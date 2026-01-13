//! DNS Upstream Clients
//!
//! Provides client implementations for querying upstream DNS servers
//! using different protocols (UDP, DoT, DoH, DoQ).

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tokio::net::UdpSocket;
use tokio::time::timeout;

use crate::dns::message::{DnsQuery, DnsResponse};
use super::upstream::{UpstreamServer, UpstreamProtocol};

/// Result of a DNS query to an upstream server
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// The DNS response
    pub response: DnsResponse,
    /// Response time in milliseconds
    pub response_time_ms: u64,
    /// Server ID that responded
    pub server_id: i64,
    /// Server name
    pub server_name: String,
}

/// Trait for DNS upstream clients
#[async_trait]
pub trait DnsClient: Send + Sync {
    /// Query the upstream server
    async fn query(&self, query: &DnsQuery) -> Result<QueryResult>;
    
    /// Get the server this client is connected to
    #[allow(dead_code)]
    fn server(&self) -> &UpstreamServer;
    
    /// Check if the server is reachable (health check)
    #[allow(dead_code)]
    async fn health_check(&self) -> Result<Duration>;
}

/// UDP DNS Client
///
/// Queries upstream DNS servers using standard UDP protocol.
pub struct UdpDnsClient {
    server: UpstreamServer,
    #[allow(dead_code)]
    socket: Option<UdpSocket>,
}

impl UdpDnsClient {
    /// Create a new UDP DNS client
    pub fn new(server: UpstreamServer) -> Self {
        Self {
            server,
            socket: None,
        }
    }

    /// Parse the server address
    fn parse_address(&self) -> Result<SocketAddr> {
        // Try to parse as socket address directly
        if let Ok(addr) = self.server.address.parse::<SocketAddr>() {
            return Ok(addr);
        }

        // Try to parse as host:port
        if self.server.address.contains(':') {
            self.server.address.parse()
                .map_err(|e| anyhow!("Invalid address format: {}", e))
        } else {
            // Assume it's just a host, add default port
            let addr = format!("{}:{}", self.server.address, UpstreamProtocol::Udp.default_port());
            addr.parse()
                .map_err(|e| anyhow!("Invalid address format: {}", e))
        }
    }

    /// Send a query and receive response
    async fn send_query(&self, query_bytes: &[u8], server_addr: SocketAddr) -> Result<Vec<u8>> {
        use tracing::debug;
        
        // Create a new socket for each query (simpler and avoids state issues)
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        
        debug!("Sending UDP query to {} ({} bytes)", server_addr, query_bytes.len());
        
        // Send query
        let sent = socket.send_to(query_bytes, server_addr).await?;
        debug!("Sent {} bytes to {}", sent, server_addr);
        
        // Receive response
        let mut buf = vec![0u8; 4096];
        debug!("Waiting for response from {} (timeout: {:?})", server_addr, self.server.timeout);
        let (len, from) = timeout(self.server.timeout, socket.recv_from(&mut buf)).await
            .map_err(|_| anyhow!("Query timeout after {:?}", self.server.timeout))??;
        
        debug!("Received {} bytes from {}", len, from);
        buf.truncate(len);
        Ok(buf)
    }
}

#[async_trait]
impl DnsClient for UdpDnsClient {
    async fn query(&self, query: &DnsQuery) -> Result<QueryResult> {
        use tracing::{debug, warn};
        
        debug!("UdpDnsClient querying {} {} via server {} ({})", 
               query.name, query.record_type, self.server.name, self.server.address);
        
        let server_addr = self.parse_address()?;
        debug!("Parsed server address: {}", server_addr);
        
        let query_bytes = query.to_bytes()
            .map_err(|e| anyhow!("Failed to encode query: {}", e))?;
        debug!("Encoded query: {} bytes", query_bytes.len());
        
        let start = Instant::now();
        let response_bytes = match self.send_query(&query_bytes, server_addr).await {
            Ok(bytes) => bytes,
            Err(e) => {
                warn!("UDP query to {} failed: {}", server_addr, e);
                return Err(e);
            }
        };
        let response_time = start.elapsed();
        
        debug!("Received response: {} bytes in {:?}", response_bytes.len(), response_time);
        
        let response = DnsResponse::from_bytes(&response_bytes)
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
        
        debug!("Parsed response: {} answers, code={}", response.answers.len(), response.response_code);
        
        Ok(QueryResult {
            response,
            response_time_ms: response_time.as_millis() as u64,
            server_id: self.server.id,
            server_name: self.server.name.clone(),
        })
    }

    fn server(&self) -> &UpstreamServer {
        &self.server
    }

    async fn health_check(&self) -> Result<Duration> {
        // Send a simple A query for a well-known domain
        let query = DnsQuery::new("dns.google", crate::dns::message::RecordType::A);
        let start = Instant::now();
        
        let _ = self.query(&query).await?;
        
        Ok(start.elapsed())
    }
}


/// DoT (DNS over TLS) Client
///
/// Queries upstream DNS servers using DNS over TLS protocol.
pub struct DotDnsClient {
    server: UpstreamServer,
}

impl DotDnsClient {
    /// Create a new DoT DNS client
    pub fn new(server: UpstreamServer) -> Self {
        Self { server }
    }

    /// Parse the server address
    fn parse_address(&self) -> Result<(String, u16)> {
        if self.server.address.contains(':') {
            let parts: Vec<&str> = self.server.address.rsplitn(2, ':').collect();
            if parts.len() == 2 {
                let port: u16 = parts[0].parse()
                    .map_err(|_| anyhow!("Invalid port"))?;
                return Ok((parts[1].to_string(), port));
            }
        }
        Ok((self.server.address.clone(), UpstreamProtocol::Dot.default_port()))
    }
}

#[async_trait]
impl DnsClient for DotDnsClient {
    async fn query(&self, query: &DnsQuery) -> Result<QueryResult> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;
        use tokio_rustls::TlsConnector;
        use rustls::{ClientConfig, RootCertStore};
        use rustls::pki_types::ServerName;
        use std::sync::Arc;

        let (host, port) = self.parse_address()?;
        let addr = format!("{}:{}", host, port);
        
        // Create TLS config with system root certificates
        let mut root_store = RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        
        let config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        
        let connector = TlsConnector::from(Arc::new(config));
        let server_name = ServerName::try_from(host.clone())
            .map_err(|_| anyhow!("Invalid server name"))?;
        
        let start = Instant::now();
        
        // Connect with timeout
        let stream = timeout(self.server.timeout, TcpStream::connect(&addr)).await
            .map_err(|_| anyhow!("Connection timeout"))??;
        
        let mut tls_stream = timeout(self.server.timeout, connector.connect(server_name, stream)).await
            .map_err(|_| anyhow!("TLS handshake timeout"))??;
        
        // Encode query with length prefix (TCP DNS format)
        let query_bytes = query.to_bytes()
            .map_err(|e| anyhow!("Failed to encode query: {}", e))?;
        let len = (query_bytes.len() as u16).to_be_bytes();
        
        tls_stream.write_all(&len).await?;
        tls_stream.write_all(&query_bytes).await?;
        tls_stream.flush().await?;
        
        // Read response length
        let mut len_buf = [0u8; 2];
        timeout(self.server.timeout, tls_stream.read_exact(&mut len_buf)).await
            .map_err(|_| anyhow!("Read timeout"))??;
        let response_len = u16::from_be_bytes(len_buf) as usize;
        
        // Read response
        let mut response_bytes = vec![0u8; response_len];
        timeout(self.server.timeout, tls_stream.read_exact(&mut response_bytes)).await
            .map_err(|_| anyhow!("Read timeout"))??;
        
        let response_time = start.elapsed();
        
        let response = DnsResponse::from_bytes(&response_bytes)
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
        
        Ok(QueryResult {
            response,
            response_time_ms: response_time.as_millis() as u64,
            server_id: self.server.id,
            server_name: self.server.name.clone(),
        })
    }

    fn server(&self) -> &UpstreamServer {
        &self.server
    }

    async fn health_check(&self) -> Result<Duration> {
        let query = DnsQuery::new("dns.google", crate::dns::message::RecordType::A);
        let start = Instant::now();
        let _ = self.query(&query).await?;
        Ok(start.elapsed())
    }
}


/// DoH (DNS over HTTPS) Client
///
/// Queries upstream DNS servers using DNS over HTTPS protocol.
pub struct DohDnsClient {
    server: UpstreamServer,
    client: reqwest::Client,
}

impl DohDnsClient {
    /// Create a new DoH DNS client
    pub fn new(server: UpstreamServer) -> Self {
        let client = reqwest::Client::builder()
            .timeout(server.timeout)
            .build()
            .unwrap_or_default();
        
        Self { server, client }
    }

    /// Get the DoH URL
    fn get_url(&self) -> String {
        if self.server.address.starts_with("http://") || self.server.address.starts_with("https://") {
            self.server.address.clone()
        } else {
            format!("https://{}/dns-query", self.server.address)
        }
    }
}

#[async_trait]
impl DnsClient for DohDnsClient {
    async fn query(&self, query: &DnsQuery) -> Result<QueryResult> {
        let url = self.get_url();
        let query_bytes = query.to_bytes()
            .map_err(|e| anyhow!("Failed to encode query: {}", e))?;
        
        let start = Instant::now();
        
        // Use POST method with application/dns-message content type
        let response = self.client
            .post(&url)
            .header("Content-Type", "application/dns-message")
            .header("Accept", "application/dns-message")
            .body(query_bytes)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("DoH query failed with status: {}", response.status()));
        }
        
        let response_bytes = response.bytes().await?;
        let response_time = start.elapsed();
        
        let dns_response = DnsResponse::from_bytes(&response_bytes)
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
        
        Ok(QueryResult {
            response: dns_response,
            response_time_ms: response_time.as_millis() as u64,
            server_id: self.server.id,
            server_name: self.server.name.clone(),
        })
    }

    fn server(&self) -> &UpstreamServer {
        &self.server
    }

    async fn health_check(&self) -> Result<Duration> {
        let query = DnsQuery::new("dns.google", crate::dns::message::RecordType::A);
        let start = Instant::now();
        let _ = self.query(&query).await?;
        Ok(start.elapsed())
    }
}

/// DoQ (DNS over QUIC) Client
///
/// Queries upstream DNS servers using DNS over QUIC protocol.
pub struct DoqDnsClient {
    server: UpstreamServer,
}

impl DoqDnsClient {
    /// Create a new DoQ DNS client
    pub fn new(server: UpstreamServer) -> Self {
        Self { server }
    }

    /// Parse the server address and resolve hostname if needed
    /// Prefers IPv4 addresses over IPv6 for better compatibility
    /// 
    /// Returns (SocketAddr, SNI) where SNI is the original host (IP or hostname)
    async fn resolve_address(&self) -> Result<(SocketAddr, String)> {
        let (host, port) = if self.server.address.contains(':') {
            // Check if it's an IPv6 address in brackets like [::1]:853
            if self.server.address.starts_with('[') {
                if let Some(bracket_end) = self.server.address.find(']') {
                    let ipv6 = &self.server.address[1..bracket_end];
                    let port_str = &self.server.address[bracket_end+1..];
                    let port: u16 = if port_str.starts_with(':') {
                        port_str[1..].parse().unwrap_or(UpstreamProtocol::Doq.default_port())
                    } else {
                        UpstreamProtocol::Doq.default_port()
                    };
                    (ipv6.to_string(), port)
                } else {
                    (self.server.address.clone(), UpstreamProtocol::Doq.default_port())
                }
            } else {
                let parts: Vec<&str> = self.server.address.rsplitn(2, ':').collect();
                if parts.len() == 2 {
                    let port: u16 = parts[0].parse()
                        .map_err(|_| anyhow!("Invalid port"))?;
                    (parts[1].to_string(), port)
                } else {
                    (self.server.address.clone(), UpstreamProtocol::Doq.default_port())
                }
            }
        } else {
            (self.server.address.clone(), UpstreamProtocol::Doq.default_port())
        };

        // Try to parse as IPv4 address first
        if let Ok(ipv4) = host.parse::<std::net::Ipv4Addr>() {
            let addr = SocketAddr::new(std::net::IpAddr::V4(ipv4), port);
            return Ok((addr, host));
        }

        // Try to parse as IPv6 address
        if let Ok(ipv6) = host.parse::<std::net::Ipv6Addr>() {
            let addr = SocketAddr::new(std::net::IpAddr::V6(ipv6), port);
            return Ok((addr, host));
        }

        // It's a hostname, resolve it - prefer IPv4
        use tokio::net::lookup_host;
        let addr_str = format!("{}:{}", host, port);
        let addrs: Vec<SocketAddr> = lookup_host(&addr_str).await
            .map_err(|e| anyhow!("Failed to resolve hostname {}: {}", host, e))?
            .collect();
        
        if addrs.is_empty() {
            return Err(anyhow!("No addresses found for {}", host));
        }

        // Prefer IPv4 addresses
        let addr = addrs.iter()
            .find(|a| a.is_ipv4())
            .or_else(|| addrs.first())
            .cloned()
            .ok_or_else(|| anyhow!("No addresses found for {}", host))?;
        
        Ok((addr, host))
    }
}

#[async_trait]
impl DnsClient for DoqDnsClient {
    async fn query(&self, query: &DnsQuery) -> Result<QueryResult> {
        use quinn::{ClientConfig, Endpoint};
        use rustls::RootCertStore;
        use std::sync::Arc;
        use tracing::debug;

        let (addr, sni_host) = self.resolve_address().await?;
        debug!("DoQ connecting to {} (SNI: {})", addr, sni_host);
        
        // Create QUIC client config
        let mut root_store = RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let mut crypto = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerifier))
            .with_no_client_auth();
        
        // Set ALPN protocol for DoQ (RFC 9250)
        crypto.alpn_protocols = vec![b"doq".to_vec()];
        
        // Create QUIC client config using quinn's crypto wrapper
        let quic_crypto = quinn::crypto::rustls::QuicClientConfig::try_from(crypto)
            .map_err(|e| anyhow!("Failed to create QUIC client config: {}", e))?;
        let client_config = ClientConfig::new(Arc::new(quic_crypto));
        
        // Bind to appropriate address based on target (IPv4 or IPv6)
        let bind_addr: SocketAddr = if addr.is_ipv6() {
            "[::]:0".parse()?
        } else {
            "0.0.0.0:0".parse()?
        };
        
        let mut endpoint = Endpoint::client(bind_addr)?;
        endpoint.set_default_client_config(client_config);
        
        let start = Instant::now();
        
        // For IP addresses, quinn's connect() requires a valid DNS name for SNI.
        // We use a placeholder domain "dns" since we're skipping certificate verification anyway.
        // This is how AdGuard dnsproxy handles IP-based DoQ connections - it uses InsecureSkipVerify
        // and doesn't rely on SNI for certificate validation.
        let connect_sni = sni_host.as_str();
        debug!("DoQ using SNI: {}", connect_sni);
        
        // Connect with timeout
        let connection = timeout(
            self.server.timeout,
            endpoint.connect(addr, connect_sni)?
        ).await
            .map_err(|_| anyhow!("Connection timeout"))??;
        
        debug!("DoQ connection established to {}", addr);
        
        // Open a bidirectional stream
        let (mut send, mut recv) = timeout(
            self.server.timeout,
            connection.open_bi()
        ).await
            .map_err(|_| anyhow!("Stream open timeout"))??;
        
        // Encode query with 2-byte length prefix (RFC 9250 Section 4.2)
        // "a 2-octet length field is used in exactly the same way as the 2-octet length field defined for DNS over TCP"
        // RFC 9250 Section 4.2.1: "When sending queries over a QUIC connection, the DNS Message ID MUST be set to 0"
        let doq_query = DnsQuery::with_id(0, &query.name, query.record_type.clone());
        let query_bytes = doq_query.to_bytes()
            .map_err(|e| anyhow!("Failed to encode query: {}", e))?;
        let len = (query_bytes.len() as u16).to_be_bytes();
        
        debug!("DoQ sending {} bytes query (with 2-byte length prefix)", query_bytes.len());
        send.write_all(&len).await?;
        send.write_all(&query_bytes).await?;
        // finish() is no longer async in quinn 0.11
        send.finish().map_err(|e| anyhow!("Failed to finish stream: {}", e))?;
        
        // Read response with 2-byte length prefix (RFC 9250)
        let mut len_buf = [0u8; 2];
        debug!("DoQ waiting for response length prefix...");
        match recv.read_exact(&mut len_buf).await {
            Ok(_) => {
                let response_len = u16::from_be_bytes(len_buf) as usize;
                debug!("DoQ response length: {} bytes", response_len);
                
                if response_len == 0 || response_len > 65535 {
                    return Err(anyhow!("Invalid response length: {}", response_len));
                }
                
                let mut response_bytes = vec![0u8; response_len];
                recv.read_exact(&mut response_bytes).await
                    .map_err(|e| anyhow!("Failed to read response body: {}", e))?;
                
                debug!("DoQ received {} bytes response", response_bytes.len());
                
                let response_time = start.elapsed();
                
                let response = DnsResponse::from_bytes(&response_bytes)
                    .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
                
                debug!("DoQ response code: {}", response.response_code);
                
                // Close connection
                connection.close(0u32.into(), b"done");
                
                Ok(QueryResult {
                    response,
                    response_time_ms: response_time.as_millis() as u64,
                    server_id: self.server.id,
                    server_name: self.server.name.clone(),
                })
            }
            Err(e) => {
                // Try reading without length prefix (some servers may not use it)
                debug!("DoQ failed to read length prefix: {}, trying without prefix...", e);
                
                // The first 2 bytes we tried to read might be part of the DNS message
                // Try to read the rest of the stream
                let mut response_bytes = len_buf.to_vec();
                match recv.read_to_end(65535).await {
                    Ok(rest) => {
                        response_bytes.extend(rest);
                        debug!("DoQ received {} bytes response (no length prefix)", response_bytes.len());
                        
                        if response_bytes.len() < 12 {
                            return Err(anyhow!("Response too short: {} bytes", response_bytes.len()));
                        }
                        
                        let response_time = start.elapsed();
                        
                        let response = DnsResponse::from_bytes(&response_bytes)
                            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
                        
                        debug!("DoQ response code: {}", response.response_code);
                        
                        connection.close(0u32.into(), b"done");
                        
                        Ok(QueryResult {
                            response,
                            response_time_ms: response_time.as_millis() as u64,
                            server_id: self.server.id,
                            server_name: self.server.name.clone(),
                        })
                    }
                    Err(e2) => {
                        Err(anyhow!("Failed to read response: length prefix error: {}, fallback error: {}", e, e2))
                    }
                }
            }
        }
    }

    fn server(&self) -> &UpstreamServer {
        &self.server
    }

    async fn health_check(&self) -> Result<Duration> {
        let query = DnsQuery::new("dns.google", crate::dns::message::RecordType::A);
        let start = Instant::now();
        let _ = self.query(&query).await?;
        Ok(start.elapsed())
    }
}

/// Certificate verifier that accepts any certificate (for IP-based connections)
#[derive(Debug)]
struct NoVerifier;

impl rustls::client::danger::ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}


/// DoH3 (DNS over HTTP/3) Client
///
/// Queries upstream DNS servers using DNS over HTTP/3 protocol.
pub struct Doh3DnsClient {
    server: UpstreamServer,
}

impl Doh3DnsClient {
    /// Create a new DoH3 DNS client
    pub fn new(server: UpstreamServer) -> Self {
        Self { server }
    }

    /// Get the DoH3 URL and parse host/port
    fn parse_url(&self) -> Result<(String, String, u16, String)> {
        let addr = &self.server.address;
        
        // Parse URL format: https://host:port/path or host:port or host
        if addr.starts_with("https://") {
            let without_scheme = &addr[8..];
            let (host_port, path) = if let Some(slash_pos) = without_scheme.find('/') {
                (&without_scheme[..slash_pos], &without_scheme[slash_pos..])
            } else {
                (without_scheme, "/dns-query")
            };
            
            let (host, port) = if host_port.contains(':') {
                let parts: Vec<&str> = host_port.rsplitn(2, ':').collect();
                let port: u16 = parts[0].parse().unwrap_or(443);
                (parts[1].to_string(), port)
            } else {
                (host_port.to_string(), 443)
            };
            
            Ok((host.clone(), host, port, path.to_string()))
        } else if addr.contains(':') {
            let parts: Vec<&str> = addr.rsplitn(2, ':').collect();
            let port: u16 = parts[0].parse().unwrap_or(443);
            let host = parts[1].to_string();
            Ok((host.clone(), host, port, "/dns-query".to_string()))
        } else {
            Ok((addr.clone(), addr.clone(), 443, "/dns-query".to_string()))
        }
    }

    /// Resolve hostname to socket address
    async fn resolve_address(&self, host: &str, port: u16) -> Result<std::net::SocketAddr> {
        // Try to parse as IP address first
        if let Ok(ip) = host.parse::<std::net::IpAddr>() {
            return Ok(std::net::SocketAddr::new(ip, port));
        }

        // Resolve hostname
        use tokio::net::lookup_host;
        let addr_str = format!("{}:{}", host, port);
        let addrs: Vec<std::net::SocketAddr> = lookup_host(&addr_str).await
            .map_err(|e| anyhow!("Failed to resolve hostname {}: {}", host, e))?
            .collect();
        
        // Prefer IPv4
        addrs.iter()
            .find(|a| a.is_ipv4())
            .or_else(|| addrs.first())
            .cloned()
            .ok_or_else(|| anyhow!("No addresses found for {}", host))
    }
}

#[async_trait]
impl DnsClient for Doh3DnsClient {
    async fn query(&self, query: &DnsQuery) -> Result<QueryResult> {
        use http::{Request, Method};
        use bytes::{Bytes, Buf};
        use tracing::debug;

        let (sni_host, host, port, path) = self.parse_url()?;
        let addr = self.resolve_address(&host, port).await?;
        
        debug!("DoH3 connecting to {} (SNI: {}, path: {})", addr, sni_host, path);

        // Create QUIC client config
        let mut root_store = rustls::RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        let is_ip = sni_host.parse::<std::net::IpAddr>().is_ok();
        
        let mut crypto = if is_ip {
            let config = rustls::ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(std::sync::Arc::new(NoVerifier))
                .with_no_client_auth();
            config
        } else {
            rustls::ClientConfig::builder()
                .with_root_certificates(root_store.clone())
                .with_no_client_auth()
        };

        // Set ALPN for HTTP/3
        crypto.alpn_protocols = vec![b"h3".to_vec()];

        // Create QUIC client config using quinn's crypto wrapper
        let quic_crypto = quinn::crypto::rustls::QuicClientConfig::try_from(crypto)
            .map_err(|e| anyhow!("Failed to create QUIC client config: {}", e))?;
        let client_config = quinn::ClientConfig::new(std::sync::Arc::new(quic_crypto));

        // Bind to appropriate address
        let bind_addr: std::net::SocketAddr = if addr.is_ipv6() {
            "[::]:0".parse()?
        } else {
            "0.0.0.0:0".parse()?
        };

        let mut endpoint = quinn::Endpoint::client(bind_addr)?;
        endpoint.set_default_client_config(client_config);

        let start = std::time::Instant::now();

        // Connect
        let connect_sni = if is_ip { "dns" } else { sni_host.as_str() };
        debug!("DoH3 using SNI: {}", connect_sni);

        let connection = timeout(
            self.server.timeout,
            endpoint.connect(addr, connect_sni)?
        ).await
            .map_err(|_| anyhow!("Connection timeout"))??;

        debug!("DoH3 QUIC connection established");

        // Create HTTP/3 connection
        let quinn_conn = h3_quinn::Connection::new(connection);
        let (mut driver, mut send_request) = h3::client::new(quinn_conn).await
            .map_err(|e| anyhow!("Failed to create H3 connection: {}", e))?;

        // Spawn driver task
        tokio::spawn(async move {
            // poll_close returns () on success or ConnectionError on failure
            let _ = futures::future::poll_fn(|cx| driver.poll_close(cx)).await;
        });

        // Encode DNS query
        let query_bytes = query.to_bytes()
            .map_err(|e| anyhow!("Failed to encode query: {}", e))?;

        // Build HTTP/3 request
        let uri = format!("https://{}:{}{}", sni_host, port, path);
        let request = Request::builder()
            .method(Method::POST)
            .uri(&uri)
            .header("content-type", "application/dns-message")
            .header("accept", "application/dns-message")
            .body(())
            .map_err(|e| anyhow!("Failed to build request: {}", e))?;

        debug!("DoH3 sending request to {}", uri);

        // Send request
        let mut stream = send_request.send_request(request).await
            .map_err(|e| anyhow!("Failed to send request: {}", e))?;

        // Send body
        stream.send_data(Bytes::from(query_bytes)).await
            .map_err(|e| anyhow!("Failed to send body: {}", e))?;

        stream.finish().await
            .map_err(|e| anyhow!("Failed to finish request: {}", e))?;

        // Receive response
        let response = stream.recv_response().await
            .map_err(|e| anyhow!("Failed to receive response: {}", e))?;

        debug!("DoH3 response status: {}", response.status());

        if !response.status().is_success() {
            return Err(anyhow!("DoH3 query failed with status: {}", response.status()));
        }

        // Read response body
        let mut response_bytes = Vec::new();
        while let Some(mut chunk) = stream.recv_data().await
            .map_err(|e| anyhow!("Failed to read response body: {}", e))? 
        {
            // Use Buf trait to copy bytes
            while chunk.has_remaining() {
                let bytes = chunk.chunk();
                response_bytes.extend_from_slice(bytes);
                chunk.advance(bytes.len());
            }
        }

        let response_time = start.elapsed();
        debug!("DoH3 received {} bytes in {:?}", response_bytes.len(), response_time);

        let dns_response = DnsResponse::from_bytes(&response_bytes)
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;

        Ok(QueryResult {
            response: dns_response,
            response_time_ms: response_time.as_millis() as u64,
            server_id: self.server.id,
            server_name: self.server.name.clone(),
        })
    }

    fn server(&self) -> &UpstreamServer {
        &self.server
    }

    async fn health_check(&self) -> Result<Duration> {
        let query = DnsQuery::new("dns.google", crate::dns::message::RecordType::A);
        let start = std::time::Instant::now();
        let _ = self.query(&query).await?;
        Ok(start.elapsed())
    }
}


/// Create a DNS client for the given upstream server
pub fn create_client(server: UpstreamServer) -> Box<dyn DnsClient> {
    match server.protocol {
        UpstreamProtocol::Udp => Box::new(UdpDnsClient::new(server)),
        UpstreamProtocol::Dot => Box::new(DotDnsClient::new(server)),
        UpstreamProtocol::Doh => Box::new(DohDnsClient::new(server)),
        UpstreamProtocol::Doq => Box::new(DoqDnsClient::new(server)),
        UpstreamProtocol::Doh3 => Box::new(Doh3DnsClient::new(server)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_udp_client() {
        let server = UpstreamServer::new(
            1, "Test", "8.8.8.8:53", UpstreamProtocol::Udp, 5000,
        );
        let client = create_client(server.clone());
        assert_eq!(client.server().protocol, UpstreamProtocol::Udp);
    }

    #[test]
    fn test_create_dot_client() {
        let server = UpstreamServer::new(
            1, "Test", "dns.google:853", UpstreamProtocol::Dot, 5000,
        );
        let client = create_client(server.clone());
        assert_eq!(client.server().protocol, UpstreamProtocol::Dot);
    }

    #[test]
    fn test_create_doh_client() {
        let server = UpstreamServer::new(
            1, "Test", "https://dns.google/dns-query", UpstreamProtocol::Doh, 5000,
        );
        let client = create_client(server.clone());
        assert_eq!(client.server().protocol, UpstreamProtocol::Doh);
    }

    #[test]
    fn test_create_doq_client() {
        let server = UpstreamServer::new(
            1, "Test", "dns.adguard.com:853", UpstreamProtocol::Doq, 5000,
        );
        let client = create_client(server.clone());
        assert_eq!(client.server().protocol, UpstreamProtocol::Doq);
    }

    #[test]
    fn test_create_doh3_client() {
        let server = UpstreamServer::new(
            1, "Test", "https://dns.adguard-dns.com/dns-query", UpstreamProtocol::Doh3, 5000,
        );
        let client = create_client(server.clone());
        assert_eq!(client.server().protocol, UpstreamProtocol::Doh3);
    }

    #[test]
    fn test_doh_url_generation() {
        let server = UpstreamServer::new(
            1, "Test", "dns.google", UpstreamProtocol::Doh, 5000,
        );
        let client = DohDnsClient::new(server);
        assert_eq!(client.get_url(), "https://dns.google/dns-query");

        let server2 = UpstreamServer::new(
            2, "Test2", "https://cloudflare-dns.com/dns-query", UpstreamProtocol::Doh, 5000,
        );
        let client2 = DohDnsClient::new(server2);
        assert_eq!(client2.get_url(), "https://cloudflare-dns.com/dns-query");
    }
}

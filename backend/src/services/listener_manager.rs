//! Listener Manager
//!
//! Manages the lifecycle of DNS server listeners (UDP, DoT, DoH, DoQ).
//! Supports dynamic starting, stopping, and restarting of listeners without application restart.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::AbortHandle;
use tracing::{info, error, warn};
use chrono::Local;

use crate::db::Database;
use crate::dns::DnsResolver;
use crate::dns::server::{UdpDnsServer, DohDnsServer, DotDnsServer, DoqDnsServer, TlsConfig};

/// Listener Manager
///
/// Handles spawning and aborting of listener tasks.
#[derive(Clone)]
pub struct ListenerManager {
    db: Arc<Database>,
    resolver: Arc<DnsResolver>,
    /// Running tasks by protocol name
    tasks: Arc<RwLock<HashMap<String, AbortHandle>>>,
}

impl ListenerManager {
    /// Create a new ListenerManager
    pub fn new(db: Arc<Database>, resolver: Arc<DnsResolver>) -> Self {
        Self {
            db,
            resolver,
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start all enabled listeners from database
    pub async fn start_all_enabled(&self) {
        info!("Starting all enabled listeners...");
        let listeners = match self.db.server_listeners().list_enabled().await {
            Ok(list) => list,
            Err(e) => {
                error!("Failed to load listeners from database: {}", e);
                return;
            }
        };

        for listener in listeners {
            if let Err(e) = self.start_listener(&listener.protocol).await {
                error!("Failed to start {} listener: {}", listener.protocol, e);
            }
        }
    }

    /// Start a specific listener by protocol
    pub async fn start_listener(&self, protocol: &str) -> anyhow::Result<()> {
        // Double check if already running
        if self.is_running(protocol).await {
            warn!("Listener {} is already running, restarting...", protocol);
            self.stop_listener(protocol).await;
        }

        // Fetch config
        let listener = match self.db.server_listeners().get_by_protocol(protocol).await {
            Ok(Some(l)) => l,
            Ok(None) => {
                let err = format!("Listener {} not found in database", protocol);
                error!("{}", err);
                return Err(anyhow::anyhow!(err));
            },
            Err(e) => {
                let err = format!("Failed to fetch listener config for {}: {}", protocol, e);
                error!("{}", err);
                return Err(anyhow::anyhow!(err));
            }
        };


        // NOTE: Removed enabled check here because the caller (listeners.rs)
        // has already verified the enabled state from the database update response.
        // Re-reading from DB here could get stale data due to transaction timing.

        let bind_addr = format!("{}:{}", listener.bind_address, listener.port);
        let addr: SocketAddr = match bind_addr.parse() {
            Ok(a) => a,
            Err(e) => {
                let err = format!("Invalid bind address for {}: {} - {}", protocol, bind_addr, e);
                error!("{}", err);
                return Err(anyhow::anyhow!(err));
            }
        };

        let resolver = self.resolver.clone();
        let _task_protocol = protocol.to_string();

        info!("Starting {} listener on {}", protocol, addr);

        let handle = match protocol {
            "udp" => {
                // Try to bind first
                match UdpDnsServer::new(addr, resolver).await {
                    Ok(server) => {
                        let msg = format!("✅ UDP listener started on {}", addr);
                        info!("{}", msg);
                        let time = Local::now().format("%Y-%m-%d %H:%M:%S");
                        println!("{} {}", time, msg);

                        let server = Arc::new(server);
                        let task = tokio::spawn(async move {
                            if let Err(e) = server.run().await {
                                error!("UDP DNS server error: {}", e);
                            }
                            info!("UDP listener stopped");
                        });
                        task.abort_handle()
                    }
                    Err(e) => {
                        error!("Failed to bind UDP server: {}", e);
                        return Err(e);
                    }
                }
            }
            "dot" => {
                if let (Some(cert), Some(key)) = (listener.tls_cert, listener.tls_key) {
                     let cert_path = format!("/tmp/fluxdns_{}_cert.pem", protocol);
                     let key_path = format!("/tmp/fluxdns_{}_key.pem", protocol);
                     
                     if let Err(e) = std::fs::write(&cert_path, cert) {
                         error!("Failed to write cert file for {}: {}", protocol, e);
                         return Err(anyhow::anyhow!(e));
                     }
                     if let Err(e) = std::fs::write(&key_path, key) {
                         error!("Failed to write key file for {}: {}", protocol, e);
                         return Err(anyhow::anyhow!(e));
                     }
                     
                     let tls_config = TlsConfig::new(cert_path, key_path);

                    match DotDnsServer::new(addr, tls_config, resolver).await {
                        Ok(server) => {
                            let msg = format!("✅ DoT listener started on {}", addr);
                            info!("{}", msg);
                            let time = Local::now().format("%Y-%m-%d %H:%M:%S");
                            println!("{} {}", time, msg);

                            let task = tokio::spawn(async move {
                                if let Err(e) = server.run().await {
                                    error!("DoT DNS server error: {}", e);
                                }
                                info!("DoT listener stopped");
                            });
                            task.abort_handle()
                        }
                        Err(e) => {
                            error!("Failed to start DoT server: {}", e);
                            return Err(e);
                        }
                    }
                } else {
                    let err = "DoT listener enabled but missing TLS certificate or key";
                    error!("{}", err);
                    return Err(anyhow::anyhow!(err));
                }
            }
            "doh" => {
                 // Check if TLS, if not, fail because DoH means HTTPS
                 let cert_pem = match &listener.tls_cert {
                     Some(c) => c.clone(),
                     None => {
                         let err = "DoH listener requires TLS certificate";
                         error!("{}", err);
                         return Err(anyhow::anyhow!(err));
                     }
                 };
                 let key_pem = match &listener.tls_key {
                     Some(k) => k.clone(),
                     None => {
                         let err = "DoH listener requires TLS private key";
                         error!("{}", err);
                         return Err(anyhow::anyhow!(err));
                     }
                 };
                 
                 // Parse certificates
                 let certs: Vec<rustls::pki_types::CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_pem.as_bytes())
                     .collect::<Result<Vec<_>, _>>()
                     .map_err(|e| anyhow::anyhow!("Failed to parse certificate: {}", e))?;
                 
                 if certs.is_empty() {
                     let err = "No valid certificates found in PEM";
                     error!("{}", err);
                     return Err(anyhow::anyhow!(err));
                 }
                 
                 // Parse private key
                 let key = rustls_pemfile::private_key(&mut key_pem.as_bytes())
                     .map_err(|e| anyhow::anyhow!("Failed to parse private key: {}", e))?
                     .ok_or_else(|| anyhow::anyhow!("No private key found in PEM"))?;
                 
                 // Build rustls config
                 let tls_config = rustls::ServerConfig::builder()
                     .with_no_client_auth()
                     .with_single_cert(certs, key)
                     .map_err(|e| anyhow::anyhow!("Failed to build TLS config: {}", e))?;
                 
                 let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(tls_config));
                 
                 // Bind TCP listener first
                 let tcp_listener = match tokio::net::TcpListener::bind(addr).await {
                     Ok(l) => l,
                     Err(e) => {
                         error!("Failed to bind DoH address {}: {}", addr, e);
                         return Err(anyhow::anyhow!(e));
                     }
                 };
                 
                 let server = DohDnsServer::new(resolver.clone());
                 let app = server.router();
                 
                 let msg = format!("✅ DoH listener (HTTPS) started on {}", addr);
                 info!("{}", msg);
                 let time = Local::now().format("%Y-%m-%d %H:%M:%S");
                 println!("{} {}", time, msg);
                 
                 let task = tokio::spawn(async move {
                     use tower::ServiceExt;
                     use hyper::server::conn::http1;
                     use hyper::service::service_fn;
                     use hyper_util::rt::TokioIo;
                     use axum::extract::ConnectInfo;
                     
                     loop {
                         let (stream, peer_addr) = match tcp_listener.accept().await {
                             Ok(s) => s,
                             Err(e) => {
                                 error!("TCP accept error: {}", e);
                                 continue;
                             }
                         };
                         
                         let acceptor = acceptor.clone();
                         let app = app.clone();
                         
                         tokio::spawn(async move {
                             match acceptor.accept(stream).await {
                                 Ok(tls_stream) => {
                                     let io = TokioIo::new(tls_stream);
                                     let svc = app.clone();
                                     let addr = peer_addr;
                                     
                                     let service = service_fn(move |mut req: hyper::Request<hyper::body::Incoming>| {
                                         let svc = svc.clone();
                                         // Inject ConnectInfo extension for client IP extraction
                                         req.extensions_mut().insert(ConnectInfo(addr));
                                         async move {
                                             svc.oneshot(req).await
                                         }
                                     });
                                     
                                     if let Err(e) = http1::Builder::new()
                                         .serve_connection(io, service)
                                         .await
                                     {
                                         // Don't log connection reset errors as they're common
                                         if !e.to_string().contains("connection reset") {
                                             tracing::debug!("HTTP connection error from {}: {}", peer_addr, e);
                                         }
                                     }
                                 }
                                 Err(e) => {
                                     tracing::debug!("TLS handshake failed from {}: {}", peer_addr, e);
                                 }
                             }
                         });
                     }
                 });
                 task.abort_handle()
            }
            "doq" => {
               if let (Some(cert), Some(key)) = (listener.tls_cert, listener.tls_key) {
                   let cert_path = format!("/tmp/fluxdns_{}_cert.pem", protocol);
                   let key_path = format!("/tmp/fluxdns_{}_key.pem", protocol);
                   std::fs::write(&cert_path, cert).unwrap_or(());
                   std::fs::write(&key_path, key).unwrap_or(());
                   let tls_config = TlsConfig::new(cert_path, key_path);

                   match DoqDnsServer::new(addr, tls_config, resolver).await {
                        Ok(server) => {
                            let msg = format!("✅ DoQ listener started on {}", addr);
                            info!("{}", msg);
                            let time = Local::now().format("%Y-%m-%d %H:%M:%S");
                            println!("{} {}", time, msg);

                            let task = tokio::spawn(async move {
                                if let Err(e) = server.run().await {
                                    error!("DoQ server error: {}", e);
                                }
                            });
                            task.abort_handle()
                        }
                        Err(e) => {
                            error!("Failed to start DoQ server: {}", e);
                            return Err(e);
                        }
                   }
               } else {
                   let err = "DoQ missing TLS config";
                   error!("{}", err);
                   return Err(anyhow::anyhow!(err));
               }
            }
            "doh3" => {
                // DoH3 (DNS over HTTP/3) requires a full HTTP/3 server implementation
                // This is more complex than DoQ as it needs HTTP/3 framing on top of QUIC
                // For now, return a clear error message
                let err = "DoH3 (DNS over HTTP/3) is not yet implemented. Consider using DoH (HTTPS) or DoQ instead.";
                error!("{}", err);
                return Err(anyhow::anyhow!(err));
            }
            _ => {
                let err = format!("Unknown protocol: {}", protocol);
                warn!("{}", err);
                return Err(anyhow::anyhow!(err));
            }
        };

        self.tasks.write().await.insert(protocol.to_string(), handle);
        Ok(())
    }

    /// Stop a specific listener
    pub async fn stop_listener(&self, protocol: &str) {
        let mut tasks = self.tasks.write().await;
        if let Some(handle) = tasks.remove(protocol) {
            handle.abort();
            let msg = format!("🛑 {} listener stopped", protocol.to_uppercase());
            info!("{}", msg);
            let time = Local::now().format("%Y-%m-%d %H:%M:%S");
            println!("{} {}", time, msg);
        }
    }



    /// Check if a listener is running
    pub async fn is_running(&self, protocol: &str) -> bool {
        self.tasks.read().await.contains_key(protocol)
    }
}

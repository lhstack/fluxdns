//! Upstream Server Management
//!
//! Manages upstream DNS servers with support for multiple protocols,
//! health checking, and statistics tracking.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::db::{Database, UpstreamServer as DbUpstreamServer};

/// Supported upstream DNS protocols
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpstreamProtocol {
    /// Standard UDP DNS (port 53)
    Udp,
    /// DNS over TLS (port 853)
    Dot,
    /// DNS over HTTPS (port 443)
    Doh,
    /// DNS over QUIC (port 853)
    Doq,
    /// DNS over HTTP/3 (port 443)
    Doh3,
}

impl UpstreamProtocol {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "udp" => Some(UpstreamProtocol::Udp),
            "dot" => Some(UpstreamProtocol::Dot),
            "doh" => Some(UpstreamProtocol::Doh),
            "doq" => Some(UpstreamProtocol::Doq),
            "doh3" | "h3" => Some(UpstreamProtocol::Doh3),
            _ => None,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            UpstreamProtocol::Udp => "udp",
            UpstreamProtocol::Dot => "dot",
            UpstreamProtocol::Doh => "doh",
            UpstreamProtocol::Doq => "doq",
            UpstreamProtocol::Doh3 => "doh3",
        }
    }

    /// Get default port for this protocol
    pub fn default_port(&self) -> u16 {
        match self {
            UpstreamProtocol::Udp => 53,
            UpstreamProtocol::Dot => 853,
            UpstreamProtocol::Doh => 443,
            UpstreamProtocol::Doq => 853,  // RFC 9250: DoQ uses UDP port 853
            UpstreamProtocol::Doh3 => 443, // DoH3 uses UDP port 443
        }
    }
}

impl std::fmt::Display for UpstreamProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}


/// Upstream server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamServer {
    /// Server ID from database
    pub id: i64,
    /// Human-readable name
    pub name: String,
    /// Server address (host:port or URL for DoH)
    pub address: String,
    /// Protocol to use
    pub protocol: UpstreamProtocol,
    /// Query timeout in milliseconds
    pub timeout: Duration,
    /// Whether this server is enabled
    pub enabled: bool,
}

impl UpstreamServer {
    /// Create a new upstream server
    pub fn new(
        id: i64,
        name: impl Into<String>,
        address: impl Into<String>,
        protocol: UpstreamProtocol,
        timeout_ms: u32,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            address: address.into(),
            protocol,
            timeout: Duration::from_millis(timeout_ms as u64),
            enabled: true,
        }
    }

    /// Create from database model
    pub fn from_db(db_server: &DbUpstreamServer) -> Option<Self> {
        let protocol = UpstreamProtocol::from_str(&db_server.protocol)?;
        Some(Self {
            id: db_server.id,
            name: db_server.name.clone(),
            address: db_server.address.clone(),
            protocol,
            timeout: Duration::from_millis(db_server.timeout as u64),
            enabled: db_server.enabled,
        })
    }

    /// Get the timeout in milliseconds
    pub fn timeout_ms(&self) -> u32 {
        self.timeout.as_millis() as u32
    }
}

/// Statistics for an upstream server
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpstreamStats {
    /// Total number of queries sent
    pub queries: u64,
    /// Number of successful responses
    pub successes: u64,
    /// Number of failures (timeouts, errors)
    pub failures: u64,
    /// Total response time in milliseconds
    pub total_response_time_ms: u64,
    /// Last response time in milliseconds
    pub last_response_time_ms: Option<u64>,
    /// Last successful query time (not serialized)
    #[serde(skip)]
    pub last_success: Option<Instant>,
    /// Last failure time (not serialized)
    #[serde(skip)]
    pub last_failure: Option<Instant>,
    /// Whether the server is currently healthy
    pub healthy: bool,
}

impl UpstreamStats {
    /// Create new stats with healthy status
    pub fn new() -> Self {
        Self {
            healthy: true,
            ..Default::default()
        }
    }

    /// Calculate average response time in milliseconds
    pub fn avg_response_time_ms(&self) -> u64 {
        if self.successes == 0 {
            0
        } else {
            self.total_response_time_ms / self.successes
        }
    }

    /// Calculate success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.queries == 0 {
            1.0 // Assume healthy if no queries yet
        } else {
            self.successes as f64 / self.queries as f64
        }
    }

    /// Record a successful query
    pub fn record_success(&mut self, response_time_ms: u64) {
        self.queries += 1;
        self.successes += 1;
        self.total_response_time_ms += response_time_ms;
        self.last_response_time_ms = Some(response_time_ms);
        self.last_success = Some(Instant::now());
        self.healthy = true;
    }

    /// Record a failed query
    pub fn record_failure(&mut self) {
        self.queries += 1;
        self.failures += 1;
        self.last_failure = Some(Instant::now());
        
        // Mark as unhealthy if failure rate is too high
        if self.success_rate() < 0.5 && self.queries >= 5 {
            self.healthy = false;
        }
    }

    /// Check if server should be considered healthy
    pub fn is_healthy(&self) -> bool {
        self.healthy
    }

    /// Reset health status (for manual recovery)
    pub fn reset_health(&mut self) {
        self.healthy = true;
    }
}


/// Health check result
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HealthCheckResult {
    /// Server ID
    pub server_id: i64,
    /// Whether the check passed
    pub healthy: bool,
    /// Response time if successful
    pub response_time_ms: Option<u64>,
    /// Error message if failed
    pub error: Option<String>,
}

/// Upstream Server Manager
///
/// Manages a collection of upstream DNS servers with health checking
/// and statistics tracking.
pub struct UpstreamManager {
    /// Loaded servers
    servers: RwLock<Vec<UpstreamServer>>,
    /// Statistics per server (keyed by server ID)
    stats: RwLock<HashMap<i64, UpstreamStats>>,
    /// Database connection for persistence
    db: Option<Arc<Database>>,
    /// Health check interval
    #[allow(dead_code)]
    health_check_interval: Duration,
}

impl UpstreamManager {
    /// Create a new upstream manager without database
    pub fn new() -> Self {
        Self {
            servers: RwLock::new(Vec::new()),
            stats: RwLock::new(HashMap::new()),
            db: None,
            health_check_interval: Duration::from_secs(30),
        }
    }

    /// Create a new upstream manager with database connection
    pub fn with_db(db: Arc<Database>) -> Self {
        Self {
            servers: RwLock::new(Vec::new()),
            stats: RwLock::new(HashMap::new()),
            db: Some(db),
            health_check_interval: Duration::from_secs(30),
        }
    }

    /// Create a new upstream manager wrapped in Arc
    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// Load servers from database
    pub async fn load_servers(&self) -> anyhow::Result<()> {
        if let Some(ref db) = self.db {
            let db_servers = db.upstream_servers().list_enabled().await?;
            let servers: Vec<UpstreamServer> = db_servers
                .iter()
                .filter_map(|s| UpstreamServer::from_db(s))
                .collect();

            // Initialize stats for each server
            let mut stats = self.stats.write().await;
            for server in &servers {
                stats.entry(server.id).or_insert_with(UpstreamStats::new);
            }

            let mut current_servers = self.servers.write().await;
            *current_servers = servers;
        }
        Ok(())
    }

    /// Reload servers from database
    pub async fn reload_servers(&self) -> anyhow::Result<()> {
        self.load_servers().await
    }

    /// Reload servers from a provided database reference
    pub async fn reload_from_db(&self, db: &Database) -> anyhow::Result<()> {
        let db_servers = db.upstream_servers().list_enabled().await?;
        let servers: Vec<UpstreamServer> = db_servers
            .iter()
            .filter_map(|s| UpstreamServer::from_db(s))
            .collect();

        // Initialize stats for each server
        let mut stats = self.stats.write().await;
        for server in &servers {
            stats.entry(server.id).or_insert_with(UpstreamStats::new);
        }

        let mut current_servers = self.servers.write().await;
        *current_servers = servers;
        Ok(())
    }

    /// Get all enabled servers
    pub async fn get_servers(&self) -> Vec<UpstreamServer> {
        self.servers.read().await.clone()
    }

    /// Get healthy servers only
    pub async fn get_healthy_servers(&self) -> Vec<UpstreamServer> {
        let servers = self.servers.read().await;
        let stats = self.stats.read().await;

        servers
            .iter()
            .filter(|s| {
                s.enabled && stats.get(&s.id).map(|st| st.is_healthy()).unwrap_or(true)
            })
            .cloned()
            .collect()
    }

    /// Get a server by ID
    pub async fn get_server(&self, id: i64) -> Option<UpstreamServer> {
        self.servers.read().await.iter().find(|s| s.id == id).cloned()
    }

    /// Add a server (in-memory only)
    pub async fn add_server(&self, server: UpstreamServer) {
        let mut servers = self.servers.write().await;
        let mut stats = self.stats.write().await;
        
        stats.entry(server.id).or_insert_with(UpstreamStats::new);
        servers.push(server);
    }

    /// Remove a server by ID
    pub async fn remove_server(&self, id: i64) {
        let mut servers = self.servers.write().await;
        let mut stats = self.stats.write().await;
        
        servers.retain(|s| s.id != id);
        stats.remove(&id);
    }

    /// Get statistics for a server
    pub async fn get_stats(&self, id: i64) -> Option<UpstreamStats> {
        self.stats.read().await.get(&id).cloned()
    }

    /// Get all statistics
    pub async fn get_all_stats(&self) -> HashMap<i64, UpstreamStats> {
        self.stats.read().await.clone()
    }

    /// Record a successful query for a server
    pub async fn record_success(&self, id: i64, response_time_ms: u64) {
        let mut stats = self.stats.write().await;
        if let Some(server_stats) = stats.get_mut(&id) {
            server_stats.record_success(response_time_ms);
        }
    }

    /// Record a failed query for a server
    pub async fn record_failure(&self, id: i64) {
        let mut stats = self.stats.write().await;
        if let Some(server_stats) = stats.get_mut(&id) {
            server_stats.record_failure();
        }
    }

    /// Reset health status for a server
    pub async fn reset_health(&self, id: i64) {
        let mut stats = self.stats.write().await;
        if let Some(server_stats) = stats.get_mut(&id) {
            server_stats.reset_health();
        }
    }

    /// Get the server with the fastest average response time
    pub async fn get_fastest_server(&self) -> Option<UpstreamServer> {
        let servers = self.get_healthy_servers().await;
        let stats = self.stats.read().await;

        servers
            .into_iter()
            .min_by_key(|s| {
                stats
                    .get(&s.id)
                    .map(|st| st.avg_response_time_ms())
                    .unwrap_or(u64::MAX)
            })
    }

    /// Get the number of servers
    pub async fn server_count(&self) -> usize {
        self.servers.read().await.len()
    }

    /// Clear all servers
    pub async fn clear(&self) {
        let mut servers = self.servers.write().await;
        let mut stats = self.stats.write().await;
        servers.clear();
        stats.clear();
    }
}

impl Default for UpstreamManager {
    fn default() -> Self {
        Self::new()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_from_str() {
        assert_eq!(UpstreamProtocol::from_str("udp"), Some(UpstreamProtocol::Udp));
        assert_eq!(UpstreamProtocol::from_str("UDP"), Some(UpstreamProtocol::Udp));
        assert_eq!(UpstreamProtocol::from_str("dot"), Some(UpstreamProtocol::Dot));
        assert_eq!(UpstreamProtocol::from_str("doh"), Some(UpstreamProtocol::Doh));
        assert_eq!(UpstreamProtocol::from_str("doq"), Some(UpstreamProtocol::Doq));
        assert_eq!(UpstreamProtocol::from_str("doh3"), Some(UpstreamProtocol::Doh3));
        assert_eq!(UpstreamProtocol::from_str("h3"), Some(UpstreamProtocol::Doh3));
        assert_eq!(UpstreamProtocol::from_str("invalid"), None);
    }

    #[test]
    fn test_protocol_default_port() {
        assert_eq!(UpstreamProtocol::Udp.default_port(), 53);
        assert_eq!(UpstreamProtocol::Dot.default_port(), 853);
        assert_eq!(UpstreamProtocol::Doh.default_port(), 443);
        assert_eq!(UpstreamProtocol::Doq.default_port(), 853);  // RFC 9250: DoQ uses UDP port 853
        assert_eq!(UpstreamProtocol::Doh3.default_port(), 443); // DoH3 uses UDP port 443
    }

    #[test]
    fn test_upstream_server_creation() {
        let server = UpstreamServer::new(
            1,
            "Cloudflare",
            "1.1.1.1:53",
            UpstreamProtocol::Udp,
            5000,
        );

        assert_eq!(server.id, 1);
        assert_eq!(server.name, "Cloudflare");
        assert_eq!(server.address, "1.1.1.1:53");
        assert_eq!(server.protocol, UpstreamProtocol::Udp);
        assert_eq!(server.timeout_ms(), 5000);
        assert!(server.enabled);
    }

    #[test]
    fn test_upstream_stats_success() {
        let mut stats = UpstreamStats::new();
        
        stats.record_success(50);
        stats.record_success(100);
        
        assert_eq!(stats.queries, 2);
        assert_eq!(stats.successes, 2);
        assert_eq!(stats.failures, 0);
        assert_eq!(stats.avg_response_time_ms(), 75);
        assert_eq!(stats.success_rate(), 1.0);
        assert!(stats.is_healthy());
    }

    #[test]
    fn test_upstream_stats_failure() {
        let mut stats = UpstreamStats::new();
        
        stats.record_failure();
        stats.record_failure();
        
        assert_eq!(stats.queries, 2);
        assert_eq!(stats.successes, 0);
        assert_eq!(stats.failures, 2);
        assert_eq!(stats.success_rate(), 0.0);
    }

    #[test]
    fn test_upstream_stats_health_degradation() {
        let mut stats = UpstreamStats::new();
        
        // Need at least 5 queries for health check
        for _ in 0..5 {
            stats.record_failure();
        }
        
        assert!(!stats.is_healthy());
    }

    #[test]
    fn test_upstream_stats_reset_health() {
        let mut stats = UpstreamStats::new();
        
        for _ in 0..5 {
            stats.record_failure();
        }
        assert!(!stats.is_healthy());
        
        stats.reset_health();
        assert!(stats.is_healthy());
    }

    #[tokio::test]
    async fn test_upstream_manager_add_server() {
        let manager = UpstreamManager::new();
        
        let server = UpstreamServer::new(
            1,
            "Test",
            "8.8.8.8:53",
            UpstreamProtocol::Udp,
            5000,
        );
        
        manager.add_server(server).await;
        
        assert_eq!(manager.server_count().await, 1);
        
        let servers = manager.get_servers().await;
        assert_eq!(servers[0].name, "Test");
    }

    #[tokio::test]
    async fn test_upstream_manager_remove_server() {
        let manager = UpstreamManager::new();
        
        manager.add_server(UpstreamServer::new(
            1, "Test1", "8.8.8.8:53", UpstreamProtocol::Udp, 5000,
        )).await;
        manager.add_server(UpstreamServer::new(
            2, "Test2", "8.8.4.4:53", UpstreamProtocol::Udp, 5000,
        )).await;
        
        manager.remove_server(1).await;
        
        assert_eq!(manager.server_count().await, 1);
        assert!(manager.get_server(1).await.is_none());
        assert!(manager.get_server(2).await.is_some());
    }

    #[tokio::test]
    async fn test_upstream_manager_stats() {
        let manager = UpstreamManager::new();
        
        manager.add_server(UpstreamServer::new(
            1, "Test", "8.8.8.8:53", UpstreamProtocol::Udp, 5000,
        )).await;
        
        manager.record_success(1, 50).await;
        manager.record_success(1, 100).await;
        
        let stats = manager.get_stats(1).await.unwrap();
        assert_eq!(stats.successes, 2);
        assert_eq!(stats.avg_response_time_ms(), 75);
    }

    #[tokio::test]
    async fn test_upstream_manager_healthy_servers() {
        let manager = UpstreamManager::new();
        
        manager.add_server(UpstreamServer::new(
            1, "Healthy", "8.8.8.8:53", UpstreamProtocol::Udp, 5000,
        )).await;
        manager.add_server(UpstreamServer::new(
            2, "Unhealthy", "8.8.4.4:53", UpstreamProtocol::Udp, 5000,
        )).await;
        
        // Make server 2 unhealthy
        for _ in 0..5 {
            manager.record_failure(2).await;
        }
        
        let healthy = manager.get_healthy_servers().await;
        assert_eq!(healthy.len(), 1);
        assert_eq!(healthy[0].id, 1);
    }

    #[tokio::test]
    async fn test_upstream_manager_fastest_server() {
        let manager = UpstreamManager::new();
        
        manager.add_server(UpstreamServer::new(
            1, "Slow", "8.8.8.8:53", UpstreamProtocol::Udp, 5000,
        )).await;
        manager.add_server(UpstreamServer::new(
            2, "Fast", "8.8.4.4:53", UpstreamProtocol::Udp, 5000,
        )).await;
        
        manager.record_success(1, 100).await;
        manager.record_success(2, 50).await;
        
        let fastest = manager.get_fastest_server().await.unwrap();
        assert_eq!(fastest.id, 2);
    }

    #[tokio::test]
    async fn test_upstream_manager_clear() {
        let manager = UpstreamManager::new();
        
        manager.add_server(UpstreamServer::new(
            1, "Test", "8.8.8.8:53", UpstreamProtocol::Udp, 5000,
        )).await;
        
        manager.clear().await;
        
        assert_eq!(manager.server_count().await, 0);
    }
}

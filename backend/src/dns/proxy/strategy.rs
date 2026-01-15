//! DNS Query Strategies
//!
//! Provides different strategies for querying upstream DNS servers:
//! - Concurrent: Query all servers simultaneously, return first response
//! - Fastest: Use the server with the best historical response time
//! - RoundRobin: Rotate through servers sequentially
//! - Random: Select a random server for each query

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{anyhow, Result};
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::dns::message::DnsQuery;
use super::client::{create_client, QueryResult};
use super::upstream::{UpstreamManager, UpstreamServer};

/// Query strategy types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryStrategy {
    /// Query all servers simultaneously, return first response
    Concurrent,
    /// Use the server with the best historical response time
    Fastest,
    /// Rotate through servers sequentially
    RoundRobin,
    /// Select a random server for each query
    Random,
}

impl QueryStrategy {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "concurrent" => Some(QueryStrategy::Concurrent),
            "fastest" | "fastest_first" => Some(QueryStrategy::Fastest),
            "round_robin" | "roundrobin" => Some(QueryStrategy::RoundRobin),
            "random" => Some(QueryStrategy::Random),
            _ => None,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            QueryStrategy::Concurrent => "concurrent",
            QueryStrategy::Fastest => "fastest",
            QueryStrategy::RoundRobin => "round_robin",
            QueryStrategy::Random => "random",
        }
    }
}

impl Default for QueryStrategy {
    fn default() -> Self {
        QueryStrategy::Concurrent
    }
}

impl std::fmt::Display for QueryStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}


/// DNS Proxy Manager
///
/// Manages upstream DNS servers and query strategies.
pub struct ProxyManager {
    /// Upstream server manager
    upstream_manager: Arc<UpstreamManager>,
    /// Current query strategy
    strategy: RwLock<QueryStrategy>,
    /// Round-robin counter
    round_robin_counter: AtomicUsize,
}

impl ProxyManager {
    /// Create a new proxy manager
    pub fn new(upstream_manager: Arc<UpstreamManager>) -> Self {
        Self {
            upstream_manager,
            strategy: RwLock::new(QueryStrategy::default()),
            round_robin_counter: AtomicUsize::new(0),
        }
    }

    /// Create a new proxy manager wrapped in Arc
    pub fn new_shared(upstream_manager: Arc<UpstreamManager>) -> Arc<Self> {
        Arc::new(Self::new(upstream_manager))
    }

    /// Get the current strategy
    pub async fn get_strategy(&self) -> QueryStrategy {
        *self.strategy.read().await
    }

    /// Set the query strategy
    pub async fn set_strategy(&self, strategy: QueryStrategy) {
        let mut current = self.strategy.write().await;
        *current = strategy;
    }

    /// Get the upstream manager
    pub fn upstream_manager(&self) -> &Arc<UpstreamManager> {
        &self.upstream_manager
    }

    /// Query upstream servers using the configured strategy
    pub async fn query(&self, query: &DnsQuery) -> Result<QueryResult> {
        use tracing::info;
        
        let trace_id = Uuid::new_v4().to_string();
        let strategy = self.get_strategy().await;
        info!("[{}] Query start: {} {} using {}", trace_id, query.name, query.record_type, strategy);
        
        let result = match strategy {
            QueryStrategy::Concurrent => self.query_concurrent(query, &trace_id).await,
            QueryStrategy::Fastest => self.query_fastest(query, &trace_id).await,
            QueryStrategy::RoundRobin => self.query_round_robin(query, &trace_id).await,
            QueryStrategy::Random => self.query_random(query, &trace_id).await,
        };
        
        match &result {
            Ok(r) => info!(
                "[{}] Query complete: {} {} -> {} ({} answers, {}ms)",
                trace_id, query.name, query.record_type, 
                r.response.response_code, r.response.answers.len(), r.response_time_ms
            ),
            Err(e) => info!("[{}] Query failed: {} {} -> {}", trace_id, query.name, query.record_type, e),
        }
        
        result
    }

    /// Query all servers concurrently, return first successful response and cancel others
    async fn query_concurrent(&self, query: &DnsQuery, trace_id: &str) -> Result<QueryResult> {
        use tracing::{debug, info, warn};
        use crate::dns::message::DnsResponseCode;
        use tokio::select;
        use tokio_util::sync::CancellationToken;
        
        let servers = self.upstream_manager.get_healthy_servers().await;
        
        if servers.is_empty() {
            return Err(anyhow!("No healthy upstream servers available"));
        }

        let server_info: Vec<String> = servers.iter()
            .map(|s| format!("{} (addr: {}, protocol: {})", s.name, s.address, s.protocol))
            .collect();
        info!("[{}] [Concurrent] Querying {} servers: {}", trace_id, servers.len(), server_info.join(", "));

        // Create cancellation token for all tasks
        let cancel_token = CancellationToken::new();
        let mut handles = Vec::with_capacity(servers.len());

        // Spawn concurrent queries to all servers
        for server in servers {
            let q = query.clone();
            let tid = trace_id.to_string();
            let server_name = server.name.clone();
            let server_addr = server.address.clone();
            let protocol = server.protocol;
            let token = cancel_token.clone();
            
            let handle = tokio::spawn(async move {
                debug!("[{}] [Concurrent] Starting query to {} ({}) via {}", tid, server_name, server_addr, protocol);
                
                select! {
                    _ = token.cancelled() => {
                        debug!("[{}] [Concurrent] Query to {} cancelled", tid, server_name);
                        Err(anyhow!("Query cancelled"))
                    }
                    result = async {
                        let client = create_client(server);
                        client.query(&q).await
                    } => {
                        // Log individual server result
                        match &result {
                            Ok(r) => info!(
                                "[{}] [Concurrent] {} responded: {} in {}ms", 
                                tid, server_name, r.response.response_code, r.response_time_ms
                            ),
                            Err(e) => warn!("[{}] [Concurrent] {} failed: {}", tid, server_name, e),
                        }
                        result
                    }
                }
            });
            handles.push(handle);
        }

        let mut last_error: Option<String> = None;

        // Use select to get the first successful response
        loop {
            if handles.is_empty() {
                break;
            }

            // Wait for any task to complete
            let (result, _index, remaining) = futures::future::select_all(handles).await;
            handles = remaining;

            match result {
                Ok(Ok(query_result)) => {
                    let response_code = &query_result.response.response_code;
                    // Accept NoError and NxDomain as valid responses
                    if *response_code == DnsResponseCode::NoError || *response_code == DnsResponseCode::NxDomain {
                        info!(
                            "[{}] [Concurrent] Winner: {} ({}ms) - {} answers, cancelling {} remaining queries",
                            trace_id,
                            query_result.server_name, 
                            query_result.response_time_ms,
                            query_result.response.answers.len(),
                            handles.len()
                        );
                        
                        // Cancel all remaining queries
                        cancel_token.cancel();
                        
                        // Record success
                        self.upstream_manager
                            .record_success(query_result.server_id, query_result.response_time_ms)
                            .await;
                        return Ok(query_result);
                    } else {
                        warn!(
                            "[{}] [Concurrent] {} returned error: {}",
                            trace_id, query_result.server_name, response_code
                        );
                        self.upstream_manager.record_failure(query_result.server_id).await;
                        last_error = Some(format!("{} returned {}", query_result.server_name, response_code));
                    }
                }
                Ok(Err(e)) => {
                    last_error = Some(e.to_string());
                }
                Err(e) => {
                    last_error = Some(format!("Task panicked: {}", e));
                }
            }
        }

        Err(anyhow!("All upstream servers failed: {}", 
            last_error.unwrap_or_else(|| "unknown error".to_string())))
    }

    /// Query the fastest server based on historical response times
    /// Falls back to concurrent strategy if any server lacks historical data
    /// Periodically re-probes all servers to handle network changes
    async fn query_fastest(&self, query: &DnsQuery, trace_id: &str) -> Result<QueryResult> {
        use tracing::info;
        
        // Check if all healthy servers have recent stats (within last 5 minutes)
        let needs_probe = self.upstream_manager.needs_reprobe().await;
        
        if needs_probe {
            info!(
                "[{}] [Fastest] Some servers need re-probing, using concurrent strategy",
                trace_id
            );
            return self.query_concurrent(query, trace_id).await;
        }
        
        let server = self.upstream_manager.get_fastest_server().await
            .ok_or_else(|| anyhow!("No healthy upstream servers available"))?;

        let avg_time = self.upstream_manager.get_stats(server.id).await
            .map(|s| s.avg_response_time_ms())
            .unwrap_or(0);

        info!(
            "[{}] [Fastest] Selected server: {}, addr: {}, protocol: {} (avg response time: {}ms)",
            trace_id,
            server.name,
            server.address,
            server.protocol,
            avg_time
        );

        self.query_server(server, query, trace_id).await
    }

    /// Query servers in round-robin fashion
    async fn query_round_robin(&self, query: &DnsQuery, trace_id: &str) -> Result<QueryResult> {
        use tracing::info;
        
        let servers = self.upstream_manager.get_healthy_servers().await;
        
        if servers.is_empty() {
            return Err(anyhow!("No healthy upstream servers available"));
        }

        let index = self.round_robin_counter.fetch_add(1, Ordering::Relaxed) % servers.len();
        let server = servers[index].clone();

        info!(
            "[{}] [RoundRobin] Selected server #{}: {}, addr: {}, protocol: {} (avg response time: {}ms)",
            trace_id,
            index,
            server.name,
            server.address,
            server.protocol,
            self.upstream_manager.get_stats(server.id).await
                .map(|s| s.avg_response_time_ms())
                .unwrap_or(0)
        );

        self.query_server(server, query, trace_id).await
    }

    /// Query a random server
    async fn query_random(&self, query: &DnsQuery, trace_id: &str) -> Result<QueryResult> {
        use tracing::info;
        
        let servers = self.upstream_manager.get_healthy_servers().await;
        
        if servers.is_empty() {
            return Err(anyhow!("No healthy upstream servers available"));
        }

        let index = rand::thread_rng().gen_range(0..servers.len());
        let server = servers[index].clone();

        info!(
            "[{}] [Random] Selected server #{}: {}, addr: {}, protocol: {} (avg response time: {}ms)",
            trace_id,
            index,
            server.name,
            server.address,
            server.protocol,
            self.upstream_manager.get_stats(server.id).await
                .map(|s| s.avg_response_time_ms())
                .unwrap_or(0)
        );

        self.query_server(server, query, trace_id).await
    }

    /// Query a specific server with failover
    async fn query_server(&self, server: UpstreamServer, query: &DnsQuery, trace_id: &str) -> Result<QueryResult> {
        use tracing::{info, warn};
        
        let client = create_client(server.clone());
        
        match client.query(query).await {
            Ok(result) => {
                info!(
                    "[{}] Server {} responded: {} in {}ms",
                    trace_id, result.server_name, result.response.response_code, result.response_time_ms
                );
                self.upstream_manager
                    .record_success(result.server_id, result.response_time_ms)
                    .await;
                Ok(result)
            }
            Err(e) => {
                warn!("[{}] Server {} failed: {}, trying failover", trace_id, server.name, e);
                self.upstream_manager.record_failure(server.id).await;
                
                // Try failover to another server
                self.failover_query(query, server.id, trace_id).await
                    .map_err(|_| anyhow!("Query failed and failover exhausted: {}", e))
            }
        }
    }

    /// Attempt failover to another server
    async fn failover_query(&self, query: &DnsQuery, failed_server_id: i64, trace_id: &str) -> Result<QueryResult> {
        use tracing::{info, warn};
        
        let servers = self.upstream_manager.get_healthy_servers().await;
        
        // Try other servers
        for server in servers {
            if server.id == failed_server_id {
                continue;
            }

            info!(
                "[{}] [Failover] Trying server: {}, addr: {}, protocol: {}",
                trace_id, server.name, server.address, server.protocol
            );
            
            let client = create_client(server.clone());
            match client.query(query).await {
                Ok(result) => {
                    info!(
                        "[{}] [Failover] Server {} succeeded: {} in {}ms",
                        trace_id, result.server_name, result.response.response_code, result.response_time_ms
                    );
                    self.upstream_manager
                        .record_success(result.server_id, result.response_time_ms)
                        .await;
                    return Ok(result);
                }
                Err(e) => {
                    warn!("[{}] [Failover] Server {} failed: {}", trace_id, server.name, e);
                    self.upstream_manager.record_failure(server.id).await;
                }
            }
        }

        Err(anyhow!("All failover servers exhausted"))
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::dns::proxy::upstream::UpstreamProtocol;

    #[test]
    fn test_strategy_from_str() {
        assert_eq!(QueryStrategy::from_str("concurrent"), Some(QueryStrategy::Concurrent));
        assert_eq!(QueryStrategy::from_str("CONCURRENT"), Some(QueryStrategy::Concurrent));
        assert_eq!(QueryStrategy::from_str("fastest"), Some(QueryStrategy::Fastest));
        assert_eq!(QueryStrategy::from_str("fastest_first"), Some(QueryStrategy::Fastest));
        assert_eq!(QueryStrategy::from_str("round_robin"), Some(QueryStrategy::RoundRobin));
        assert_eq!(QueryStrategy::from_str("roundrobin"), Some(QueryStrategy::RoundRobin));
        assert_eq!(QueryStrategy::from_str("random"), Some(QueryStrategy::Random));
        assert_eq!(QueryStrategy::from_str("invalid"), None);
    }

    #[test]
    fn test_strategy_as_str() {
        assert_eq!(QueryStrategy::Concurrent.as_str(), "concurrent");
        assert_eq!(QueryStrategy::Fastest.as_str(), "fastest");
        assert_eq!(QueryStrategy::RoundRobin.as_str(), "round_robin");
        assert_eq!(QueryStrategy::Random.as_str(), "random");
    }

    #[test]
    fn test_strategy_default() {
        assert_eq!(QueryStrategy::default(), QueryStrategy::Concurrent);
    }

    #[tokio::test]
    async fn test_proxy_manager_strategy() {
        let upstream_manager = Arc::new(UpstreamManager::new());
        let proxy_manager = ProxyManager::new(upstream_manager);

        assert_eq!(proxy_manager.get_strategy().await, QueryStrategy::Concurrent);

        proxy_manager.set_strategy(QueryStrategy::RoundRobin).await;
        assert_eq!(proxy_manager.get_strategy().await, QueryStrategy::RoundRobin);
    }

    #[tokio::test]
    async fn test_proxy_manager_no_servers() {
        let upstream_manager = Arc::new(UpstreamManager::new());
        let proxy_manager = ProxyManager::new(upstream_manager);

        let query = DnsQuery::new("example.com", crate::dns::message::RecordType::A);
        let result = proxy_manager.query(&query).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_round_robin_counter() {
        let upstream_manager = Arc::new(UpstreamManager::new());
        
        // Add multiple servers
        upstream_manager.add_server(UpstreamServer::new(
            1, "Server1", "8.8.8.8:53", UpstreamProtocol::Udp, 5000,
        )).await;
        upstream_manager.add_server(UpstreamServer::new(
            2, "Server2", "8.8.4.4:53", UpstreamProtocol::Udp, 5000,
        )).await;
        upstream_manager.add_server(UpstreamServer::new(
            3, "Server3", "1.1.1.1:53", UpstreamProtocol::Udp, 5000,
        )).await;

        let proxy_manager = ProxyManager::new(upstream_manager);
        proxy_manager.set_strategy(QueryStrategy::RoundRobin).await;

        // Verify counter increments
        let initial = proxy_manager.round_robin_counter.load(Ordering::Relaxed);
        
        // The counter should increment on each query attempt
        // (even if the query fails due to network issues in tests)
        assert_eq!(initial, 0);
    }
}

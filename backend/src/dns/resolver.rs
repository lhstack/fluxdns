//! DNS Resolver
//!
//! The DNS Resolver integrates the rewrite engine, cache, and proxy manager
//! to provide a complete DNS resolution pipeline.

use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use tracing::{debug, info};

use crate::db::{Database, CreateQueryLog};
use super::cache::{CacheKey, CacheManager};
use super::message::{DnsQuery, DnsRecordData, DnsResponse, DnsResponseCode, RecordType};
use super::proxy::ProxyManager;
use super::rewrite::{RewriteAction, RewriteEngine};

/// Query metadata returned alongside the DNS response
#[derive(Debug, Clone)]
pub struct QueryMetadata {
    /// Total response time in milliseconds
    pub response_time_ms: u64,
    /// Whether the response was served from cache
    pub cache_hit: bool,
    /// Name of the upstream server used (if any)
    pub upstream_used: Option<String>,
    /// Whether a rewrite rule was applied
    pub rewrite_applied: bool,
    /// The rewrite rule ID that was applied (if any)
    pub rewrite_rule_id: Option<i64>,
}

impl Default for QueryMetadata {
    fn default() -> Self {
        Self {
            response_time_ms: 0,
            cache_hit: false,
            upstream_used: None,
            rewrite_applied: false,
            rewrite_rule_id: None,
        }
    }
}

/// Result of a DNS resolution
#[derive(Debug, Clone)]
pub struct ResolveResult {
    /// The DNS response
    pub response: DnsResponse,
    /// Query metadata
    pub metadata: QueryMetadata,
}

/// DNS Resolver
///
/// Integrates rewrite engine, cache, and proxy manager to provide
/// complete DNS resolution functionality.
pub struct DnsResolver {
    /// Rewrite engine for domain rewriting
    rewrite_engine: Arc<RewriteEngine>,
    /// Cache manager for caching responses
    cache: Arc<CacheManager>,
    /// Proxy manager for upstream queries
    proxy: Arc<ProxyManager>,
    /// Database for query logging (optional)
    db: Option<Arc<Database>>,
}


impl DnsResolver {
    /// Create a new DNS resolver
    pub fn new(
        rewrite_engine: Arc<RewriteEngine>,
        cache: Arc<CacheManager>,
        proxy: Arc<ProxyManager>,
    ) -> Self {
        Self {
            rewrite_engine,
            cache,
            proxy,
            db: None,
        }
    }

    /// Create a new DNS resolver with database for logging
    pub fn with_db(
        rewrite_engine: Arc<RewriteEngine>,
        cache: Arc<CacheManager>,
        proxy: Arc<ProxyManager>,
        db: Arc<Database>,
    ) -> Self {
        Self {
            rewrite_engine,
            cache,
            proxy,
            db: Some(db),
        }
    }

    /// Create a new DNS resolver wrapped in Arc
    pub fn new_shared(
        rewrite_engine: Arc<RewriteEngine>,
        cache: Arc<CacheManager>,
        proxy: Arc<ProxyManager>,
    ) -> Arc<Self> {
        Arc::new(Self::new(rewrite_engine, cache, proxy))
    }

    /// Get the rewrite engine
    pub fn rewrite_engine(&self) -> &Arc<RewriteEngine> {
        &self.rewrite_engine
    }

    /// Get the cache manager
    pub fn cache(&self) -> &Arc<CacheManager> {
        &self.cache
    }

    /// Get the proxy manager
    pub fn proxy(&self) -> &Arc<ProxyManager> {
        &self.proxy
    }

    /// Resolve a DNS query
    ///
    /// This is the main entry point for DNS resolution. It follows this flow:
    /// 1. Check if record type is disabled
    /// 2. Check rewrite rules
    /// 3. If rewrite matches, apply the action
    /// 4. Check local DNS records from database
    /// 5. Otherwise, check cache
    /// 6. If cache miss, query upstream via proxy
    /// 7. Cache the response
    pub async fn resolve(&self, query: &DnsQuery) -> Result<ResolveResult> {
        let start = Instant::now();
        let mut metadata = QueryMetadata::default();

        info!("[DNS Query] {} {} (ID: {})", query.name, query.record_type, query.id);

        // Step 0: Check if record type is disabled
        if let Some(ref db) = self.db {
            if self.is_record_type_disabled(db, &query.record_type.to_string()).await {
                info!(
                    "[DNS Result] {} {} | Disabled record type | {}ms",
                    query.name, query.record_type, start.elapsed().as_millis()
                );
                metadata.response_time_ms = start.elapsed().as_millis() as u64;
                return Ok(ResolveResult {
                    response: DnsResponse::nxdomain(query.id),
                    metadata,
                });
            }
        }

        // Step 1: Check rewrite rules
        if let Some(rewrite_result) = self.rewrite_engine.check(&query.name).await {
            metadata.rewrite_applied = true;
            metadata.rewrite_rule_id = Some(rewrite_result.rule_id);

            let response = self.apply_rewrite_action(query, &rewrite_result.action).await?;
            metadata.response_time_ms = start.elapsed().as_millis() as u64;

            let action_desc = match &rewrite_result.action {
                RewriteAction::Block => "BLOCKED".to_string(),
                RewriteAction::MapToIp(ip) => format!("-> {}", ip),
                RewriteAction::MapToDomain(domain) => format!("-> {}", domain),
            };
            info!(
                "[DNS Result] {} {} | Rewrite(rule_id={}) {} | {}ms",
                query.name, query.record_type, rewrite_result.rule_id, action_desc, metadata.response_time_ms
            );

            return Ok(ResolveResult { response, metadata });
        }

        // Step 2: Check local DNS records from database
        if let Some(ref db) = self.db {
            if let Some(response) = self.check_local_records(db, query).await? {
                metadata.response_time_ms = start.elapsed().as_millis() as u64;
                let answers: Vec<String> = response.answers.iter().map(|a| a.value.clone()).collect();
                info!(
                    "[DNS Result] {} {} | LocalRecord | {} | {}ms",
                    query.name, query.record_type, answers.join(", "), metadata.response_time_ms
                );
                return Ok(ResolveResult { response, metadata });
            }
        }

        // Step 3: Check cache
        let cache_key = CacheKey::from_query(query);
        if let Some(cached_response) = self.cache.get(&cache_key).await {
            metadata.cache_hit = true;
            metadata.response_time_ms = start.elapsed().as_millis() as u64;

            // Update response ID to match query
            let mut response = cached_response;
            response.id = query.id;

            let answers: Vec<String> = response.answers.iter().map(|a| a.value.clone()).collect();
            info!(
                "[DNS Result] {} {} | Cache | {} | {}ms",
                query.name, query.record_type, answers.join(", "), metadata.response_time_ms
            );

            return Ok(ResolveResult { response, metadata });
        }

        debug!("Cache miss for {} {}", query.name, query.record_type);

        // Step 4: Query upstream via proxy
        let query_result = self.proxy.query(query).await?;
        
        metadata.upstream_used = Some(query_result.server_name.clone());
        metadata.response_time_ms = start.elapsed().as_millis() as u64;

        // Restore original query ID in response (important for DoQ which uses ID=0)
        let mut response = query_result.response;
        response.id = query.id;

        // Step 5: Cache the response (only if successful)
        if response.response_code == DnsResponseCode::NoError {
            self.cache.set(cache_key, response.clone()).await;
        }

        let answers: Vec<String> = response.answers.iter().map(|a| a.value.clone()).collect();
        let result_str = if answers.is_empty() {
            format!("{}", response.response_code)
        } else {
            answers.join(", ")
        };
        info!(
            "[DNS Result] {} {} | Upstream({}) | {} | {}ms",
            query.name, query.record_type, query_result.server_name, result_str, metadata.response_time_ms
        );

        Ok(ResolveResult {
            response,
            metadata,
        })
    }

    /// Check if a record type is disabled in settings
    async fn is_record_type_disabled(&self, db: &Database, record_type: &str) -> bool {
        match db.system_config().get("disabled_record_types").await {
            Ok(Some(value)) => {
                if let Ok(disabled_types) = serde_json::from_str::<Vec<String>>(&value) {
                    let upper = record_type.to_uppercase();
                    return disabled_types.iter().any(|t| t.to_uppercase() == upper);
                }
                false
            }
            _ => false,
        }
    }

    /// Resolve a DNS query with client IP for logging
    ///
    /// This method wraps resolve() and saves the query log to database.
    pub async fn resolve_with_client(&self, query: &DnsQuery, client_ip: &str) -> Result<ResolveResult> {
        let result = self.resolve(query).await;
        
        // Save query log to database (fire and forget)
        if let Some(ref db) = self.db {
            let log = match &result {
                Ok(r) => CreateQueryLog {
                    client_ip: client_ip.to_string(),
                    query_name: query.name.clone(),
                    query_type: query.record_type.to_string(),
                    response_code: Some(r.response.response_code.to_string()),
                    response_time: Some(r.metadata.response_time_ms as i32),
                    cache_hit: r.metadata.cache_hit,
                    upstream_used: r.metadata.upstream_used.clone(),
                },
                Err(e) => CreateQueryLog {
                    client_ip: client_ip.to_string(),
                    query_name: query.name.clone(),
                    query_type: query.record_type.to_string(),
                    response_code: Some(format!("ERROR: {}", e)),
                    response_time: None,
                    cache_hit: false,
                    upstream_used: None,
                },
            };
            
            let db = db.clone();
            tokio::spawn(async move {
                if let Err(e) = db.query_logs().create(log).await {
                    tracing::warn!("Failed to save query log: {}", e);
                }
            });
        }
        
        result
    }

    /// Check local DNS records from database
    async fn check_local_records(&self, db: &Database, query: &DnsQuery) -> Result<Option<DnsResponse>> {
        use std::net::{Ipv4Addr, Ipv6Addr};
        use std::str::FromStr;

        let record_type_str = query.record_type.to_string();
        // Use wildcard-aware query method
        let records = db.dns_records().get_by_name_and_type_with_wildcard(&query.name, &record_type_str).await?;

        if records.is_empty() {
            return Ok(None);
        }

        let mut response = DnsResponse::new(query.id);

        for record in records {
            if !record.enabled {
                continue;
            }

            // For wildcard records, use the queried name instead of the record name
            let response_name = if record.name.starts_with("*.") {
                &query.name
            } else {
                &record.name
            };

            let dns_record = match query.record_type {
                RecordType::A => {
                    if let Ok(ip) = Ipv4Addr::from_str(&record.value) {
                        Some(DnsRecordData::a(response_name, ip, record.ttl as u32))
                    } else {
                        debug!("Invalid IPv4 address in DNS record: {}", record.value);
                        None
                    }
                }
                RecordType::AAAA => {
                    if let Ok(ip) = Ipv6Addr::from_str(&record.value) {
                        Some(DnsRecordData::aaaa(response_name, ip, record.ttl as u32))
                    } else {
                        debug!("Invalid IPv6 address in DNS record: {}", record.value);
                        None
                    }
                }
                RecordType::CNAME => {
                    Some(DnsRecordData::cname(response_name, &record.value, record.ttl as u32))
                }
                RecordType::MX => {
                    Some(DnsRecordData::mx(response_name, &record.value, record.priority as u16, record.ttl as u32))
                }
                RecordType::TXT => {
                    Some(DnsRecordData::txt(response_name, &record.value, record.ttl as u32))
                }
                RecordType::PTR => {
                    Some(DnsRecordData::ptr(response_name, &record.value, record.ttl as u32))
                }
                RecordType::NS => {
                    Some(DnsRecordData::ns(response_name, &record.value, record.ttl as u32))
                }
                _ => None,
            };

            if let Some(dns_record) = dns_record {
                response.add_answer(dns_record);
            }
        }

        if response.answers.is_empty() {
            Ok(None)
        } else {
            Ok(Some(response))
        }
    }

    /// Resolve a DNS query by name and record type
    pub async fn resolve_with_type(
        &self,
        name: &str,
        record_type: RecordType,
    ) -> Result<ResolveResult> {
        let query = DnsQuery::new(name, record_type);
        self.resolve(&query).await
    }

    /// Apply a rewrite action and generate a response
    async fn apply_rewrite_action(
        &self,
        query: &DnsQuery,
        action: &RewriteAction,
    ) -> Result<DnsResponse> {
        match action {
            RewriteAction::MapToIp(ip) => {
                self.create_ip_response(query, *ip)
            }
            RewriteAction::MapToDomain(target_domain) => {
                // Resolve the target domain
                let target_query = DnsQuery::new(target_domain, query.record_type);
                let result = self.resolve_without_rewrite(&target_query).await?;
                
                // Return response with original query ID
                let mut response = result.response;
                response.id = query.id;
                Ok(response)
            }
            RewriteAction::Block => {
                Ok(DnsResponse::nxdomain(query.id))
            }
        }
    }

    /// Create a response with an IP address
    fn create_ip_response(&self, query: &DnsQuery, ip: IpAddr) -> Result<DnsResponse> {
        let mut response = DnsResponse::new(query.id);
        
        match (ip, query.record_type) {
            (IpAddr::V4(ipv4), RecordType::A) => {
                response.add_answer(DnsRecordData::a(&query.name, ipv4, 300));
            }
            (IpAddr::V6(ipv6), RecordType::AAAA) => {
                response.add_answer(DnsRecordData::aaaa(&query.name, ipv6, 300));
            }
            (IpAddr::V4(ipv4), RecordType::AAAA) => {
                // Requested AAAA but have IPv4 - return empty response
                debug!(
                    "Rewrite has IPv4 {} but query is for AAAA record",
                    ipv4
                );
            }
            (IpAddr::V6(ipv6), RecordType::A) => {
                // Requested A but have IPv6 - return empty response
                debug!(
                    "Rewrite has IPv6 {} but query is for A record",
                    ipv6
                );
            }
            _ => {
                // For other record types with IP rewrite, return empty response
                debug!(
                    "Rewrite has IP but query is for {} record",
                    query.record_type
                );
            }
        }

        Ok(response)
    }

    /// Resolve without checking rewrite rules (to avoid infinite loops)
    /// This is kept for backward compatibility but now delegates to resolve_with_depth
    async fn resolve_without_rewrite(&self, query: &DnsQuery) -> Result<ResolveResult> {
        // Start with depth 1 since we're already in a rewrite
        self.resolve_with_depth(query, 1).await
    }

    /// Resolve with depth tracking to prevent infinite loops
    /// max_depth is 10 to allow reasonable chaining while preventing infinite loops
    fn resolve_with_depth<'a>(
        &'a self,
        query: &'a DnsQuery,
        depth: u32,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ResolveResult>> + Send + 'a>> {
        Box::pin(async move {
            const MAX_DEPTH: u32 = 10;
            
            if depth > MAX_DEPTH {
                debug!("Max rewrite depth {} exceeded for {}", MAX_DEPTH, query.name);
                return Err(anyhow::anyhow!("Max rewrite depth exceeded, possible circular reference"));
            }

            let start = Instant::now();
            let mut metadata = QueryMetadata::default();

            debug!(
                "Resolving DNS query (depth {}): {} {} (ID: {})",
                depth, query.name, query.record_type, query.id
            );

            // Step 1: Check rewrite rules (allow chaining)
            if let Some(rewrite_result) = self.rewrite_engine.check(&query.name).await {
                debug!(
                    "Rewrite rule {} matched for {} (depth {})",
                    rewrite_result.rule_id, query.name, depth
                );
                metadata.rewrite_applied = true;
                metadata.rewrite_rule_id = Some(rewrite_result.rule_id);

                let response = self.apply_rewrite_action_with_depth(query, &rewrite_result.action, depth).await?;
                metadata.response_time_ms = start.elapsed().as_millis() as u64;

                return Ok(ResolveResult { response, metadata });
            }

            // Step 2: Check local DNS records from database
            if let Some(ref db) = self.db {
                if let Some(response) = self.check_local_records(db, query).await? {
                    debug!("Local DNS record found for {} {} (depth {})", query.name, query.record_type, depth);
                    metadata.response_time_ms = start.elapsed().as_millis() as u64;
                    return Ok(ResolveResult { response, metadata });
                }
            }

            // Step 3: Check cache
            let cache_key = CacheKey::from_query(query);
            if let Some(cached_response) = self.cache.get(&cache_key).await {
                metadata.cache_hit = true;
                metadata.response_time_ms = start.elapsed().as_millis() as u64;

                let mut response = cached_response;
                response.id = query.id;

                return Ok(ResolveResult { response, metadata });
            }

            // Step 4: Query upstream
            let query_result = self.proxy.query(query).await?;
            
            metadata.upstream_used = Some(query_result.server_name);
            metadata.response_time_ms = start.elapsed().as_millis() as u64;

            // Restore original query ID in response (important for DoQ which uses ID=0)
            let mut response = query_result.response;
            response.id = query.id;

            // Cache the response
            if response.response_code == DnsResponseCode::NoError {
                self.cache.set(cache_key, response.clone()).await;
            }

            Ok(ResolveResult {
                response,
                metadata,
            })
        })
    }

    /// Apply a rewrite action with depth tracking
    fn apply_rewrite_action_with_depth<'a>(
        &'a self,
        query: &'a DnsQuery,
        action: &'a RewriteAction,
        depth: u32,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<DnsResponse>> + Send + 'a>> {
        Box::pin(async move {
            match action {
                RewriteAction::MapToIp(ip) => {
                    self.create_ip_response(query, *ip)
                }
                RewriteAction::MapToDomain(target_domain) => {
                    // Resolve the target domain with increased depth
                    let target_query = DnsQuery::new(target_domain, query.record_type);
                    let result = self.resolve_with_depth(&target_query, depth + 1).await?;
                    
                    // Return response with original query ID
                    let mut response = result.response;
                    response.id = query.id;
                    Ok(response)
                }
                RewriteAction::Block => {
                    Ok(DnsResponse::nxdomain(query.id))
                }
            }
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::dns::cache::CacheConfig;
    use crate::dns::proxy::{UpstreamManager, UpstreamProtocol, UpstreamServer};
    use crate::dns::rewrite::{MatchType, RewriteRule};
    use std::net::Ipv4Addr;

    fn create_test_resolver() -> DnsResolver {
        let rewrite_engine = Arc::new(RewriteEngine::new());
        let cache = Arc::new(CacheManager::with_config(CacheConfig {
            default_ttl: 60,
            max_entries: 1000,
        }));
        let upstream_manager = Arc::new(UpstreamManager::new());
        let proxy = Arc::new(ProxyManager::new(upstream_manager));

        DnsResolver::new(rewrite_engine, cache, proxy)
    }

    #[tokio::test]
    async fn test_resolver_creation() {
        let resolver = create_test_resolver();
        assert!(resolver.db.is_none());
    }

    #[tokio::test]
    async fn test_resolver_rewrite_block() {
        let resolver = create_test_resolver();

        // Add a block rule
        resolver.rewrite_engine.add_rule(RewriteRule::new(
            1,
            "blocked.com".to_string(),
            MatchType::Exact,
            RewriteAction::Block,
            10,
        )).await;

        let query = DnsQuery::new("blocked.com", RecordType::A);
        let result = resolver.resolve(&query).await.unwrap();

        assert_eq!(result.response.response_code, DnsResponseCode::NxDomain);
        assert!(result.metadata.rewrite_applied);
        assert_eq!(result.metadata.rewrite_rule_id, Some(1));
    }

    #[tokio::test]
    async fn test_resolver_rewrite_map_to_ip() {
        let resolver = create_test_resolver();

        // Add a map-to-ip rule
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        resolver.rewrite_engine.add_rule(RewriteRule::new(
            1,
            "local.test".to_string(),
            MatchType::Exact,
            RewriteAction::MapToIp(ip),
            10,
        )).await;

        let query = DnsQuery::new("local.test", RecordType::A);
        let result = resolver.resolve(&query).await.unwrap();

        assert_eq!(result.response.response_code, DnsResponseCode::NoError);
        assert!(result.metadata.rewrite_applied);
        assert_eq!(result.response.answers.len(), 1);
        assert_eq!(result.response.answers[0].value, "127.0.0.1");
    }

    #[tokio::test]
    async fn test_resolver_cache_hit() {
        let resolver = create_test_resolver();

        // Pre-populate cache
        let cache_key = CacheKey::new("cached.com", RecordType::A);
        let mut response = DnsResponse::new(12345);
        response.add_answer(DnsRecordData::a(
            "cached.com",
            Ipv4Addr::new(1, 2, 3, 4),
            300,
        ));
        resolver.cache.set(cache_key, response).await;

        // Query should hit cache
        let query = DnsQuery::new("cached.com", RecordType::A);
        let result = resolver.resolve(&query).await.unwrap();

        assert!(result.metadata.cache_hit);
        assert!(!result.metadata.rewrite_applied);
        assert!(result.metadata.upstream_used.is_none());
        assert_eq!(result.response.answers.len(), 1);
    }

    #[tokio::test]
    async fn test_resolver_no_upstream_servers() {
        let resolver = create_test_resolver();

        // Query without any upstream servers should fail
        let query = DnsQuery::new("example.com", RecordType::A);
        let result = resolver.resolve(&query).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_ip_response_a_record() {
        let resolver = create_test_resolver();
        let query = DnsQuery::new("test.com", RecordType::A);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        let response = resolver.create_ip_response(&query, ip).unwrap();

        assert_eq!(response.response_code, DnsResponseCode::NoError);
        assert_eq!(response.answers.len(), 1);
        assert_eq!(response.answers[0].record_type, RecordType::A);
        assert_eq!(response.answers[0].value, "192.168.1.1");
    }

    #[tokio::test]
    async fn test_create_ip_response_mismatched_type() {
        let resolver = create_test_resolver();
        let query = DnsQuery::new("test.com", RecordType::AAAA);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        let response = resolver.create_ip_response(&query, ip).unwrap();

        // Should return empty response when IP type doesn't match query type
        assert_eq!(response.response_code, DnsResponseCode::NoError);
        assert_eq!(response.answers.len(), 0);
    }

    #[tokio::test]
    async fn test_query_metadata_default() {
        let metadata = QueryMetadata::default();
        
        assert_eq!(metadata.response_time_ms, 0);
        assert!(!metadata.cache_hit);
        assert!(metadata.upstream_used.is_none());
        assert!(!metadata.rewrite_applied);
        assert!(metadata.rewrite_rule_id.is_none());
    }
}

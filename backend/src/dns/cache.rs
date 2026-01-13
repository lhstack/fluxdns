//! DNS Cache Manager
//!
//! Provides caching functionality for DNS responses with TTL-based expiration,
//! cache statistics, and cache management operations.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::message::{DnsQuery, DnsResponse, RecordType};

/// Cache key for DNS queries
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CacheKey {
    /// Domain name (lowercase)
    pub name: String,
    /// Record type
    pub record_type: RecordType,
}

impl CacheKey {
    /// Create a new cache key
    pub fn new(name: impl Into<String>, record_type: RecordType) -> Self {
        Self {
            name: name.into().to_lowercase(),
            record_type,
        }
    }

    /// Create a cache key from a DNS query
    pub fn from_query(query: &DnsQuery) -> Self {
        Self::new(&query.name, query.record_type)
    }
}

/// A cached DNS response entry
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// The cached DNS response
    pub response: DnsResponse,
    /// When this entry expires
    pub expires_at: Instant,
    /// When this entry was created
    pub created_at: Instant,
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(response: DnsResponse, ttl: Duration) -> Self {
        let now = Instant::now();
        Self {
            response,
            expires_at: now + ttl,
            created_at: now,
        }
    }

    /// Check if this entry has expired
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }

    /// Get the remaining TTL in seconds
    #[allow(dead_code)]
    pub fn remaining_ttl(&self) -> u64 {
        let now = Instant::now();
        if now >= self.expires_at {
            0
        } else {
            (self.expires_at - now).as_secs()
        }
    }
}


/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Default TTL for cached entries in seconds
    pub default_ttl: u64,
    /// Maximum number of entries in the cache
    pub max_entries: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: 60,
            max_entries: 10000,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Current number of entries in the cache
    pub entries: usize,
}

impl CacheStats {
    /// Calculate the cache hit rate (0.0 to 1.0)
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// DNS Cache Manager
///
/// Thread-safe cache for DNS responses with TTL-based expiration.
pub struct CacheManager {
    /// The cache storage
    cache: RwLock<HashMap<CacheKey, CacheEntry>>,
    /// Cache configuration
    config: RwLock<CacheConfig>,
    /// Cache statistics
    stats: RwLock<CacheStats>,
}

impl CacheManager {
    /// Create a new cache manager with default configuration
    pub fn new() -> Self {
        Self::with_config(CacheConfig::default())
    }

    /// Create a new cache manager with custom configuration
    pub fn with_config(config: CacheConfig) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            config: RwLock::new(config),
            stats: RwLock::new(CacheStats::default()),
        }
    }

    /// Create a new cache manager wrapped in Arc
    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// Get a cached response for the given key
    pub async fn get(&self, key: &CacheKey) -> Option<DnsResponse> {
        let cache = self.cache.read().await;
        
        if let Some(entry) = cache.get(key) {
            if !entry.is_expired() {
                // Update hit count
                let mut stats = self.stats.write().await;
                stats.hits += 1;
                
                return Some(entry.response.clone());
            }
        }
        
        // Update miss count
        let mut stats = self.stats.write().await;
        stats.misses += 1;
        
        None
    }

    /// Store a response in the cache
    pub async fn set(&self, key: CacheKey, response: DnsResponse) {
        let config = self.config.read().await;
        let ttl = Duration::from_secs(config.default_ttl);
        let max_entries = config.max_entries;
        drop(config);

        self.set_with_ttl(key, response, ttl, max_entries).await;
    }

    /// Store a response in the cache with a specific TTL
    pub async fn set_with_ttl(&self, key: CacheKey, response: DnsResponse, ttl: Duration, max_entries: usize) {
        let mut cache = self.cache.write().await;
        
        // Evict expired entries if we're at capacity
        if cache.len() >= max_entries {
            self.evict_expired_entries(&mut cache);
        }
        
        // If still at capacity, remove oldest entry
        if cache.len() >= max_entries {
            self.evict_oldest_entry(&mut cache);
        }
        
        let entry = CacheEntry::new(response, ttl);
        cache.insert(key, entry);
        
        // Update entry count
        let mut stats = self.stats.write().await;
        stats.entries = cache.len();
    }

    /// Clear all entries from the cache
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        
        let mut stats = self.stats.write().await;
        stats.entries = 0;
    }

    /// Clear cache entries for a specific domain
    pub async fn clear_domain(&self, domain: &str) {
        let domain_lower = domain.to_lowercase();
        let mut cache = self.cache.write().await;
        
        cache.retain(|key, _| !key.name.eq_ignore_ascii_case(&domain_lower));
        
        let mut stats = self.stats.write().await;
        stats.entries = cache.len();
    }

    /// Get current cache statistics
    pub async fn stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        let mut stats = self.stats.read().await.clone();
        stats.entries = cache.len();
        stats
    }

    /// Update the default TTL
    pub async fn set_ttl(&self, ttl_seconds: u64) {
        let mut config = self.config.write().await;
        config.default_ttl = ttl_seconds;
    }

    /// Get the current default TTL
    pub async fn get_ttl(&self) -> u64 {
        let config = self.config.read().await;
        config.default_ttl
    }

    /// Update the maximum number of entries
    pub async fn set_max_entries(&self, max_entries: usize) {
        let mut config = self.config.write().await;
        config.max_entries = max_entries;
    }

    /// Get the current configuration
    pub async fn get_config(&self) -> CacheConfig {
        self.config.read().await.clone()
    }

    /// Update the configuration
    pub async fn update_config(&self, config: CacheConfig) {
        let mut current = self.config.write().await;
        *current = config;
    }

    /// Remove expired entries from the cache
    pub async fn cleanup_expired(&self) {
        let mut cache = self.cache.write().await;
        self.evict_expired_entries(&mut cache);
        
        let mut stats = self.stats.write().await;
        stats.entries = cache.len();
    }

    /// Evict expired entries (internal helper)
    fn evict_expired_entries(&self, cache: &mut HashMap<CacheKey, CacheEntry>) {
        cache.retain(|_, entry| !entry.is_expired());
    }

    /// Evict the oldest entry (internal helper)
    fn evict_oldest_entry(&self, cache: &mut HashMap<CacheKey, CacheEntry>) {
        if let Some(oldest_key) = cache
            .iter()
            .min_by_key(|(_, entry)| entry.created_at)
            .map(|(key, _)| key.clone())
        {
            cache.remove(&oldest_key);
        }
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::dns::message::{DnsRecordData, DnsResponse, RecordType};

    fn create_test_response(id: u16) -> DnsResponse {
        let mut response = DnsResponse::new(id);
        response.add_answer(DnsRecordData::a(
            "example.com",
            "93.184.216.34".parse().unwrap(),
            300,
        ));
        response
    }

    #[tokio::test]
    async fn test_cache_set_and_get() {
        let cache = CacheManager::new();
        let key = CacheKey::new("example.com", RecordType::A);
        let response = create_test_response(12345);

        cache.set(key.clone(), response.clone()).await;
        
        let cached = cache.get(&key).await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().id, 12345);
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = CacheManager::new();
        let key = CacheKey::new("nonexistent.com", RecordType::A);
        
        let cached = cache.get(&key).await;
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let config = CacheConfig {
            default_ttl: 0, // Immediate expiration
            max_entries: 100,
        };
        let cache = CacheManager::with_config(config);
        let key = CacheKey::new("example.com", RecordType::A);
        let response = create_test_response(12345);

        cache.set(key.clone(), response).await;
        
        // Should be expired immediately
        tokio::time::sleep(Duration::from_millis(10)).await;
        let cached = cache.get(&key).await;
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let cache = CacheManager::new();
        let key1 = CacheKey::new("example1.com", RecordType::A);
        let key2 = CacheKey::new("example2.com", RecordType::A);

        cache.set(key1.clone(), create_test_response(1)).await;
        cache.set(key2.clone(), create_test_response(2)).await;

        cache.clear().await;

        assert!(cache.get(&key1).await.is_none());
        assert!(cache.get(&key2).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_clear_domain() {
        let cache = CacheManager::new();
        let key1 = CacheKey::new("example.com", RecordType::A);
        let key2 = CacheKey::new("example.com", RecordType::AAAA);
        let key3 = CacheKey::new("other.com", RecordType::A);

        cache.set(key1.clone(), create_test_response(1)).await;
        cache.set(key2.clone(), create_test_response(2)).await;
        cache.set(key3.clone(), create_test_response(3)).await;

        cache.clear_domain("example.com").await;

        assert!(cache.get(&key1).await.is_none());
        assert!(cache.get(&key2).await.is_none());
        assert!(cache.get(&key3).await.is_some());
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = CacheManager::new();
        let key = CacheKey::new("example.com", RecordType::A);
        let response = create_test_response(12345);

        // Miss
        cache.get(&key).await;
        
        // Set
        cache.set(key.clone(), response).await;
        
        // Hit
        cache.get(&key).await;
        cache.get(&key).await;

        let stats = cache.stats().await;
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.entries, 1);
        assert!((stats.hit_rate() - 0.666).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_cache_ttl_update() {
        let cache = CacheManager::new();
        
        cache.set_ttl(120).await;
        assert_eq!(cache.get_ttl().await, 120);
    }

    #[tokio::test]
    async fn test_cache_key_case_insensitive() {
        let cache = CacheManager::new();
        let key1 = CacheKey::new("EXAMPLE.COM", RecordType::A);
        let key2 = CacheKey::new("example.com", RecordType::A);

        cache.set(key1.clone(), create_test_response(12345)).await;
        
        let cached = cache.get(&key2).await;
        assert!(cached.is_some());
    }

    #[tokio::test]
    async fn test_cache_max_entries() {
        let config = CacheConfig {
            default_ttl: 60,
            max_entries: 2,
        };
        let cache = CacheManager::with_config(config);

        cache.set(CacheKey::new("a.com", RecordType::A), create_test_response(1)).await;
        cache.set(CacheKey::new("b.com", RecordType::A), create_test_response(2)).await;
        cache.set(CacheKey::new("c.com", RecordType::A), create_test_response(3)).await;

        let stats = cache.stats().await;
        assert!(stats.entries <= 2);
    }

    #[test]
    fn test_cache_entry_expiration() {
        let response = create_test_response(12345);
        let entry = CacheEntry::new(response, Duration::from_secs(0));
        
        std::thread::sleep(Duration::from_millis(10));
        assert!(entry.is_expired());
    }

    #[test]
    fn test_cache_entry_remaining_ttl() {
        let response = create_test_response(12345);
        let entry = CacheEntry::new(response, Duration::from_secs(60));
        
        assert!(entry.remaining_ttl() <= 60);
        assert!(entry.remaining_ttl() > 0);
    }

    #[test]
    fn test_cache_stats_hit_rate_zero() {
        let stats = CacheStats::default();
        assert_eq!(stats.hit_rate(), 0.0);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use crate::dns::message::{DnsRecordData, DnsResponse, RecordType};
    use proptest::prelude::*;
    use std::net::Ipv4Addr;

    /// Strategy to generate valid domain names
    fn domain_strategy() -> impl Strategy<Value = String> {
        let label = "[a-z][a-z0-9]{0,9}";
        (label, label).prop_map(|(l1, l2)| format!("{}.{}", l1, l2))
    }

    /// Strategy to generate record types
    fn record_type_strategy() -> impl Strategy<Value = RecordType> {
        prop_oneof![
            Just(RecordType::A),
            Just(RecordType::AAAA),
            Just(RecordType::CNAME),
            Just(RecordType::MX),
            Just(RecordType::TXT),
            Just(RecordType::NS),
        ]
    }

    /// Create a test response with given ID and IP
    fn create_response(id: u16, ip_octets: (u8, u8, u8, u8)) -> DnsResponse {
        let mut response = DnsResponse::new(id);
        let ip = Ipv4Addr::new(ip_octets.0, ip_octets.1, ip_octets.2, ip_octets.3);
        response.add_answer(DnsRecordData::a("test.com", ip, 300));
        response
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 6: 缓存命中一致性
        /// For any cached DNS response that has not expired, repeated queries should
        /// return the exact same response without triggering upstream queries.
        /// **Validates: Requirements 3.15**
        #[test]
        fn prop_cache_hit_consistency(
            domain in domain_strategy(),
            record_type in record_type_strategy(),
            response_id in any::<u16>(),
            ip_octets in (any::<u8>(), any::<u8>(), any::<u8>(), any::<u8>())
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let config = CacheConfig {
                    default_ttl: 3600, // Long TTL to ensure no expiration
                    max_entries: 1000,
                };
                let cache = CacheManager::with_config(config);
                let key = CacheKey::new(&domain, record_type);
                let response = create_response(response_id, ip_octets);

                // Store in cache
                cache.set(key.clone(), response.clone()).await;

                // Multiple reads should return the same response
                for _ in 0..5 {
                    let cached = cache.get(&key).await;
                    prop_assert!(cached.is_some(), "Cache should return a value for cached key");
                    let cached = cached.unwrap();
                    prop_assert_eq!(cached.id, response_id, "Response ID should be consistent");
                    prop_assert_eq!(cached.answers.len(), response.answers.len(), "Answer count should be consistent");
                }

                Ok(())
            })?;
        }

        /// Property 6: Cache key case insensitivity
        /// For any domain name, cache lookups should be case-insensitive.
        /// **Validates: Requirements 3.15**
        #[test]
        fn prop_cache_key_case_insensitive(
            domain in domain_strategy(),
            record_type in record_type_strategy(),
            response_id in any::<u16>(),
            ip_octets in (any::<u8>(), any::<u8>(), any::<u8>(), any::<u8>())
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let cache = CacheManager::new();
                let response = create_response(response_id, ip_octets);

                // Store with lowercase key
                let key_lower = CacheKey::new(&domain.to_lowercase(), record_type);
                cache.set(key_lower, response.clone()).await;

                // Retrieve with uppercase key
                let key_upper = CacheKey::new(&domain.to_uppercase(), record_type);
                let cached = cache.get(&key_upper).await;

                prop_assert!(cached.is_some(), "Cache should be case-insensitive");
                prop_assert_eq!(cached.unwrap().id, response_id, "Response should match");

                Ok(())
            })?;
        }

        /// Property 7: 缓存过期更新正确性
        /// For any expired cache entry, the next query should return None (cache miss),
        /// indicating that an upstream query is needed.
        /// **Validates: Requirements 3.16**
        #[test]
        fn prop_cache_expiration_triggers_miss(
            domain in domain_strategy(),
            record_type in record_type_strategy(),
            response_id in any::<u16>(),
            ip_octets in (any::<u8>(), any::<u8>(), any::<u8>(), any::<u8>())
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let config = CacheConfig {
                    default_ttl: 0, // Immediate expiration
                    max_entries: 1000,
                };
                let cache = CacheManager::with_config(config);
                let key = CacheKey::new(&domain, record_type);
                let response = create_response(response_id, ip_octets);

                // Store in cache
                cache.set(key.clone(), response).await;

                // Wait for expiration
                tokio::time::sleep(Duration::from_millis(10)).await;

                // Should be a cache miss after expiration
                let cached = cache.get(&key).await;
                prop_assert!(cached.is_none(), "Expired entries should result in cache miss");

                Ok(())
            })?;
        }

        /// Property 6: Cache stats consistency
        /// For any sequence of cache operations, the hit rate should be correctly calculated.
        /// **Validates: Requirements 3.15**
        #[test]
        fn prop_cache_stats_consistency(
            domain in domain_strategy(),
            record_type in record_type_strategy(),
            response_id in any::<u16>(),
            ip_octets in (any::<u8>(), any::<u8>(), any::<u8>(), any::<u8>()),
            num_hits in 1usize..10usize,
            num_misses in 1usize..10usize
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let cache = CacheManager::new();
                let key = CacheKey::new(&domain, record_type);
                let response = create_response(response_id, ip_octets);

                // Generate misses first (before caching)
                let miss_key = CacheKey::new("nonexistent.domain", RecordType::A);
                for _ in 0..num_misses {
                    cache.get(&miss_key).await;
                }

                // Store and generate hits
                cache.set(key.clone(), response).await;
                for _ in 0..num_hits {
                    cache.get(&key).await;
                }

                let stats = cache.stats().await;
                prop_assert_eq!(stats.hits as usize, num_hits, "Hit count should match");
                prop_assert_eq!(stats.misses as usize, num_misses, "Miss count should match");

                let expected_rate = num_hits as f64 / (num_hits + num_misses) as f64;
                let actual_rate = stats.hit_rate();
                prop_assert!((actual_rate - expected_rate).abs() < 0.01, 
                    "Hit rate should be correctly calculated");

                Ok(())
            })?;
        }

        /// Property 7: Cache update replaces old value
        /// For any cache key, setting a new value should replace the old one.
        /// **Validates: Requirements 3.16**
        #[test]
        fn prop_cache_update_replaces_value(
            domain in domain_strategy(),
            record_type in record_type_strategy(),
            old_id in any::<u16>(),
            new_id in any::<u16>(),
            ip_octets in (any::<u8>(), any::<u8>(), any::<u8>(), any::<u8>())
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let cache = CacheManager::new();
                let key = CacheKey::new(&domain, record_type);

                // Store old value
                let old_response = create_response(old_id, ip_octets);
                cache.set(key.clone(), old_response).await;

                // Store new value (simulating cache update after expiration)
                let new_response = create_response(new_id, ip_octets);
                cache.set(key.clone(), new_response).await;

                // Should get new value
                let cached = cache.get(&key).await;
                prop_assert!(cached.is_some(), "Cache should have the updated value");
                prop_assert_eq!(cached.unwrap().id, new_id, "Cache should return the new value");

                Ok(())
            })?;
        }
    }
}

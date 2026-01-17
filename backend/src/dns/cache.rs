//! DNS Cache Manager
//!
//! Provides caching functionality for DNS responses with TTL-based expiration,
//! cache statistics, and cache management operations.
//! 
//! Optimized with DashMap for high concurrency and approximated LRU for eviction.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::message::{DnsQuery, DnsResponse, RecordType};

/// Cache key for DNS queries
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CacheKey {
    /// Domain name (lowercase, shared string)
    pub name: Arc<str>,
    /// Record type
    pub record_type: RecordType,
}

impl CacheKey {
    /// Create a new cache key
    pub fn new(name: impl AsRef<str>, record_type: RecordType) -> Self {
        Self {
            name: Arc::from(name.as_ref().to_lowercase().as_str()),
            record_type,
        }
    }

    /// Create a cache key from a DNS query
    pub fn from_query(query: &DnsQuery) -> Self {
        Self::new(&query.name, query.record_type)
    }
}

/// A cached DNS response entry
#[derive(Debug)]
#[allow(dead_code)]
pub struct CacheEntry {
    /// The cached DNS response
    pub response: DnsResponse,
    /// When this entry expires
    pub expires_at: Instant,
    /// When this entry was created
    pub created_at: Instant,
    /// Last access timestamp (Unix timestamp in milliseconds) for LRU
    pub last_accessed: AtomicI64,
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(response: DnsResponse, ttl: Duration) -> Self {
        let now = Instant::now();
        Self {
            response,
            expires_at: now + ttl,
            created_at: now,
            last_accessed: AtomicI64::new(Self::now_millis()),
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

    /// Update last accessed time
    pub fn touch(&self) {
        self.last_accessed.store(Self::now_millis(), Ordering::Relaxed);
    }

    /// Get current time in milliseconds
    fn now_millis() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
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
/// Uses DashMap for high concurrency and approximated LRU for eviction.
pub struct CacheManager {
    /// The cache storage
    cache: DashMap<CacheKey, CacheEntry>,
    /// Cache configuration
    config: RwLock<CacheConfig>,
    /// Cache statistics - hits
    hits: AtomicU64,
    /// Cache statistics - misses
    misses: AtomicU64,
}

impl CacheManager {
    /// Create a new cache manager with default configuration
    pub fn new() -> Self {
        Self::with_config(CacheConfig::default())
    }

    /// Create a new cache manager with custom configuration
    pub fn with_config(config: CacheConfig) -> Self {
        Self {
            cache: DashMap::new(),
            config: RwLock::new(config),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Create a new cache manager wrapped in Arc
    #[allow(dead_code)]
    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// Get a cached response for the given key
    pub async fn get(&self, key: &CacheKey) -> Option<DnsResponse> {
        if let Some(entry) = self.cache.get(key) {
            if !entry.is_expired() {
                // Update hit count
                self.hits.fetch_add(1, Ordering::Relaxed);
                // Update access time for LRU
                entry.touch();
                
                return Some(entry.response.clone());
            }
        }
        
        // Update miss count
        self.misses.fetch_add(1, Ordering::Relaxed);
        
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
        // Eviction logic: if nearing capacity, perform random sampling eviction
        // We do this check loosely to avoid contention
        if self.cache.len() >= max_entries {
            self.perform_eviction(max_entries);
        }
        
        let entry = CacheEntry::new(response, ttl);
        self.cache.insert(key, entry);
    }

    /// Perform approximated LRU eviction using random sampling
    fn perform_eviction(&self, target_size: usize) {
        // If we are significantly over limit, we need to remove items
        // Sampling count: 5 random items
        const SAMPLE_SIZE: usize = 5;
        
        // We try to remove items until we are below target
        // But to avoid blocking, we only try a few times per insert
        let mut attempts = 0;
        
        while self.cache.len() >= target_size && attempts < 5 {
            // DashMap doesn't support random sampling directly efficiently without locking shards
            // But we can iterate and pick first N, or rely on internal implementation details.
            // Since we upgrade to DashMap, we can't easily get "random" keys without scanning.
            // Strategy: Quick scan of a few keys and pick oldest.
            
            // Note: In DashMap, iteration can be slow if map is huge. 
            // Better approach for strict performance: skip detailed LRU if not critical, 
            // OR just remove a few random keys if over limit.
            
            // Simple random eviction for now:
            // Just satisfy the constraint by removing some entries.
            // Ideally we'd pick the "oldest accessed" from a sample.
            
            // To emulate "random sample", we can't easily jump to a random index in DashMap.
            // We will just iterate and check a small batch.
            
            let mut candidates: Vec<(CacheKey, i64)> = Vec::with_capacity(SAMPLE_SIZE);
            
            // Collect candidates
            // We limit iteration to just finding SAMPLE_SIZE items
            for r in self.cache.iter() {
                candidates.push((r.key().clone(), r.value().last_accessed.load(Ordering::Relaxed)));
                if candidates.len() >= SAMPLE_SIZE {
                    break;
                }
            }
            
            if candidates.is_empty() {
                break;
            }
            
            // Find oldest among candidates
            if let Some((oldest_key, _)) = candidates.iter().min_by_key(|(_, t)| *t) {
                self.cache.remove(oldest_key);
            }
            
            attempts += 1;
        }
    }

    /// Clear all entries from the cache
    pub async fn clear(&self) {
        self.cache.clear();
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
    }

    /// Clear cache entries for a specific domain
    pub async fn clear_domain(&self, domain: &str) {
        let domain_lower = domain.to_lowercase();
        self.cache.retain(|key, _| !key.name.eq_ignore_ascii_case(&domain_lower));
    }

    /// Get current cache statistics
    pub async fn stats(&self) -> CacheStats {
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            entries: self.cache.len(),
        }
    }

    /// Update the default TTL
    #[allow(dead_code)]
    pub async fn set_ttl(&self, ttl_seconds: u64) {
        let mut config = self.config.write().await;
        config.default_ttl = ttl_seconds;
    }

    /// Get the current default TTL
    #[allow(dead_code)]
    pub async fn get_ttl(&self) -> u64 {
        let config = self.config.read().await;
        config.default_ttl
    }

    /// Update the maximum number of entries
    #[allow(dead_code)]
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
        self.cache.retain(|_, entry| !entry.is_expired());
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
}

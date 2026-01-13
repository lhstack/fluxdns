//! Cache Management API module
//!
//! Implements REST API endpoints for DNS cache management.
//!
//! # Requirements
//!
//! - 3.17: Provide cache TTL configuration interface
//! - 3.18: Provide manual cache clearing functionality
//! - 3.19: Provide clearing cache for specific domain
//! - 3.20: Provide clearing all cache
//! - 3.21: Display cache statistics

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::dns::{CacheConfig, CacheManager, CacheStats};
use crate::web::ApiError;

/// Application state for cache API
#[derive(Clone)]
pub struct CacheState {
    pub cache: Arc<CacheManager>,
    pub db: Arc<Database>,
}

/// Cache statistics response
#[derive(Debug, Serialize)]
pub struct CacheStatsResponse {
    pub hits: u64,
    pub misses: u64,
    pub entries: usize,
    pub hit_rate: f64,
}

impl From<CacheStats> for CacheStatsResponse {
    fn from(stats: CacheStats) -> Self {
        Self {
            hits: stats.hits,
            misses: stats.misses,
            entries: stats.entries,
            hit_rate: stats.hit_rate(),
        }
    }
}

/// Cache configuration response
#[derive(Debug, Serialize)]
pub struct CacheConfigResponse {
    pub default_ttl: u64,
    pub max_entries: usize,
}

impl From<CacheConfig> for CacheConfigResponse {
    fn from(config: CacheConfig) -> Self {
        Self {
            default_ttl: config.default_ttl,
            max_entries: config.max_entries,
        }
    }
}

/// Update cache configuration request
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateCacheConfigRequest {
    pub default_ttl: Option<u64>,
    pub max_entries: Option<usize>,
}

/// Validation error details
#[derive(Debug, Serialize)]
pub struct ValidationErrors {
    pub errors: Vec<ValidationError>,
}

#[derive(Debug, Serialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl UpdateCacheConfigRequest {
    /// Validate the update request
    pub fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = Vec::new();

        if let Some(ttl) = self.default_ttl {
            if ttl == 0 {
                errors.push(ValidationError {
                    field: "default_ttl".to_string(),
                    message: "TTL must be greater than 0".to_string(),
                });
            }
            if ttl > 86400 * 7 {
                errors.push(ValidationError {
                    field: "default_ttl".to_string(),
                    message: "TTL cannot exceed 7 days (604800 seconds)".to_string(),
                });
            }
        }

        if let Some(max_entries) = self.max_entries {
            if max_entries == 0 {
                errors.push(ValidationError {
                    field: "max_entries".to_string(),
                    message: "Max entries must be greater than 0".to_string(),
                });
            }
            if max_entries > 1_000_000 {
                errors.push(ValidationError {
                    field: "max_entries".to_string(),
                    message: "Max entries cannot exceed 1,000,000".to_string(),
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationErrors { errors })
        }
    }
}

/// Clear cache request for specific domain
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ClearDomainRequest {
    pub domain: String,
}

/// Get cache statistics
///
/// GET /api/cache/stats
pub async fn cache_stats(
    State(state): State<CacheState>,
) -> Result<impl IntoResponse, ApiError> {
    let stats = state.cache.stats().await;
    Ok(Json(CacheStatsResponse::from(stats)))
}

/// Get cache configuration
///
/// GET /api/cache/config
pub async fn get_cache_config(
    State(state): State<CacheState>,
) -> Result<impl IntoResponse, ApiError> {
    let config = state.cache.get_config().await;
    Ok(Json(CacheConfigResponse::from(config)))
}

/// Update cache configuration
///
/// PUT /api/cache/config
pub async fn update_cache_config(
    State(state): State<CacheState>,
    Json(request): Json<UpdateCacheConfigRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate request
    if let Err(validation_errors) = request.validate() {
        return Err(ApiError {
            code: "BAD_REQUEST".to_string(),
            message: "Validation failed".to_string(),
            details: Some(serde_json::to_value(validation_errors).unwrap()),
        });
    }

    let mut config = state.cache.get_config().await;

    if let Some(ttl) = request.default_ttl {
        config.default_ttl = ttl;
    }
    if let Some(max_entries) = request.max_entries {
        config.max_entries = max_entries;
    }

    state.cache.update_config(config.clone()).await;

    // Persist to database
    let sys_config = state.db.system_config();
    if let Err(e) = sys_config.set("cache_default_ttl", &config.default_ttl.to_string()).await {
        tracing::warn!("Failed to persist cache_default_ttl: {}", e);
    }
    if let Err(e) = sys_config.set("cache_max_entries", &config.max_entries.to_string()).await {
        tracing::warn!("Failed to persist cache_max_entries: {}", e);
    }

    tracing::info!("Cache config updated: ttl={}, max_entries={}", config.default_ttl, config.max_entries);

    Ok(Json(CacheConfigResponse::from(config)))
}

/// Clear all cache entries
///
/// POST /api/cache/clear
pub async fn clear_cache(
    State(state): State<CacheState>,
) -> Result<impl IntoResponse, ApiError> {
    state.cache.clear().await;

    Ok(Json(serde_json::json!({
        "message": "Cache cleared successfully"
    })))
}

/// Clear cache entries for a specific domain
///
/// POST /api/cache/clear/:domain
pub async fn clear_domain_cache(
    State(state): State<CacheState>,
    Path(domain): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    if domain.is_empty() {
        return Err(ApiError {
            code: "BAD_REQUEST".to_string(),
            message: "Domain cannot be empty".to_string(),
            details: None,
        });
    }

    state.cache.clear_domain(&domain).await;

    Ok(Json(serde_json::json!({
        "message": format!("Cache cleared for domain: {}", domain)
    })))
}

/// Cleanup expired cache entries
///
/// POST /api/cache/cleanup
pub async fn cleanup_cache(
    State(state): State<CacheState>,
) -> Result<impl IntoResponse, ApiError> {
    state.cache.cleanup_expired().await;

    let stats = state.cache.stats().await;

    Ok(Json(serde_json::json!({
        "message": "Expired cache entries cleaned up",
        "remaining_entries": stats.entries
    })))
}

/// Build the cache API router
pub fn cache_router(state: CacheState) -> axum::Router {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/stats", get(cache_stats))
        .route("/config", get(get_cache_config).put(update_cache_config))
        .route("/clear", post(clear_cache))
        .route("/clear/:domain", post(clear_domain_cache))
        .route("/cleanup", post(cleanup_cache))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_stats_response_from() {
        let stats = CacheStats {
            hits: 100,
            misses: 50,
            entries: 25,
        };
        let response = CacheStatsResponse::from(stats);
        assert_eq!(response.hits, 100);
        assert_eq!(response.misses, 50);
        assert_eq!(response.entries, 25);
        assert!((response.hit_rate - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_cache_config_response_from() {
        let config = CacheConfig {
            default_ttl: 60,
            max_entries: 10000,
        };
        let response = CacheConfigResponse::from(config);
        assert_eq!(response.default_ttl, 60);
        assert_eq!(response.max_entries, 10000);
    }

    #[test]
    fn test_update_cache_config_validation_valid() {
        let request = UpdateCacheConfigRequest {
            default_ttl: Some(300),
            max_entries: Some(5000),
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_update_cache_config_validation_invalid_ttl() {
        let request = UpdateCacheConfigRequest {
            default_ttl: Some(0),
            max_entries: None,
        };
        assert!(request.validate().is_err());

        let request = UpdateCacheConfigRequest {
            default_ttl: Some(86400 * 8), // More than 7 days
            max_entries: None,
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_update_cache_config_validation_invalid_max_entries() {
        let request = UpdateCacheConfigRequest {
            default_ttl: None,
            max_entries: Some(0),
        };
        assert!(request.validate().is_err());

        let request = UpdateCacheConfigRequest {
            default_ttl: None,
            max_entries: Some(1_000_001),
        };
        assert!(request.validate().is_err());
    }
}

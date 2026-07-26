//! System Status API module
//!
//! Implements REST API endpoint for system status monitoring.
//!
//! # Requirements
//!
//! - 4.6: Provide service status monitoring functionality

use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;

use crate::application::web::ApiError;
use crate::business::status_business::StatusBusiness;

/// Application state for status API
#[derive(Clone)]
pub struct StatusState {
    pub business: Arc<StatusBusiness>,
}

/// System status response
#[derive(Debug, Serialize)]
pub struct SystemStatusResponse {
    pub status: String,
    pub uptime_seconds: u64,
    pub cache: CacheStatusInfo,
    pub query: QueryStatusInfo,
    pub upstreams: UpstreamsStatusInfo,
    pub strategy: String,
}

/// Cache status information
#[derive(Debug, Serialize)]
pub struct CacheStatusInfo {
    pub entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub default_ttl: u64,
    pub max_entries: usize,
}

/// Query status information
#[derive(Debug, Serialize)]
pub struct QueryStatusInfo {
    pub total_queries: i64,
    pub cache_hits: i64,
    pub queries_today: i64,
    /// Query log entries discarded because the write queue was full.
    ///
    /// Non-zero means logging could not keep up with query volume; resolution
    /// itself is unaffected, but the log is incomplete.
    pub dropped_log_entries: u64,
}

/// Upstreams status information
#[derive(Debug, Serialize)]
pub struct UpstreamsStatusInfo {
    pub total: usize,
    pub healthy: usize,
    pub servers: Vec<UpstreamStatusInfo>,
}

/// Individual upstream server status
#[derive(Debug, Serialize)]
pub struct UpstreamStatusInfo {
    pub id: i64,
    pub name: String,
    pub address: String,
    pub protocol: String,
    pub enabled: bool,
    pub healthy: bool,
    pub queries: u64,
    pub failures: u64,
    pub avg_response_time_ms: u64,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub database: bool,
    pub upstreams: bool,
}

/// Get system status
///
/// GET /api/status
/// Get system status
///
/// GET /api/status
pub async fn system_status(
    State(state): State<StatusState>,
) -> Result<impl IntoResponse, ApiError> {
    let status = state.business.system_status().await?;

    Ok(Json(SystemStatusResponse {
        status: "running".to_string(),
        uptime_seconds: status.uptime_seconds,
        cache: CacheStatusInfo {
            entries: status.cache_stats.entries,
            hits: status.cache_stats.hits,
            misses: status.cache_stats.misses,
            hit_rate: status.cache_stats.hit_rate(),
            default_ttl: status.cache_config.default_ttl,
            max_entries: status.cache_config.max_entries,
        },
        query: QueryStatusInfo {
            total_queries: status.query_stats.total_queries,
            cache_hits: status.query_stats.cache_hits,
            queries_today: status.query_stats.queries_today,
            dropped_log_entries: status.dropped_query_logs,
        },
        upstreams: UpstreamsStatusInfo {
            total: status.upstreams.len(),
            healthy: status.healthy_upstreams,
            servers: status
                .upstreams
                .into_iter()
                .map(|u| UpstreamStatusInfo {
                    id: u.id,
                    name: u.name,
                    address: u.address,
                    protocol: u.protocol,
                    enabled: u.enabled,
                    healthy: u.healthy,
                    queries: u.queries,
                    failures: u.failures,
                    avg_response_time_ms: u.avg_response_time_ms,
                })
                .collect(),
        },
        strategy: status.strategy.to_string(),
    }))
}

/// Health check endpoint
///
/// GET /api/status/health
pub async fn health_check(State(state): State<StatusState>) -> Result<impl IntoResponse, ApiError> {
    let report = state.business.health_report().await;

    Ok(Json(HealthCheckResponse {
        status: report.status().to_string(),
        database: report.database,
        upstreams: report.upstreams,
    }))
}

/// Build the status API router
pub fn status_router(state: StatusState) -> axum::Router {
    use axum::routing::get;

    axum::Router::new()
        .route("/", get(system_status))
        .route("/health", get(health_check))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_status_info() {
        let info = CacheStatusInfo {
            entries: 100,
            hits: 80,
            misses: 20,
            hit_rate: 0.8,
            default_ttl: 60,
            max_entries: 10000,
        };
        assert_eq!(info.entries, 100);
        assert_eq!(info.hit_rate, 0.8);
    }

    #[test]
    fn test_query_status_info() {
        let info = QueryStatusInfo {
            total_queries: 1000,
            cache_hits: 750,
            queries_today: 100,
            dropped_log_entries: 0,
        };
        assert_eq!(info.total_queries, 1000);
    }

    #[test]
    fn test_health_check_response() {
        let response = HealthCheckResponse {
            status: "healthy".to_string(),
            database: true,
            upstreams: true,
        };
        assert_eq!(response.status, "healthy");
        assert!(response.database);
    }
}

//! Query Logs API module
//!
//! Implements REST API endpoints for DNS query log viewing.
//!
//! # Requirements
//!
//! - 4.5: Provide query log viewing functionality

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::application::web::ApiError;
use crate::business::log_business::LogBusiness;
use crate::infrastructure::repository::{PaginatedResult, QueryLog, QueryLogFilter, QueryStats};

/// Upper bound on exported rows, to keep a single export from exhausting memory.
const EXPORT_MAX_ROWS: i64 = 10_000;

/// Application state for logs API
#[derive(Clone)]
pub struct LogsState {
    pub business: Arc<LogBusiness>,
}

/// Query parameters for log listing
#[derive(Debug, Clone, Deserialize)]
pub struct LogsQueryParams {
    pub query_name: Option<String>,
    pub query_type: Option<String>,
    pub client_ip: Option<String>,
    pub cache_hit: Option<bool>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub format: Option<String>,
}

impl From<LogsQueryParams> for QueryLogFilter {
    fn from(params: LogsQueryParams) -> Self {
        Self {
            query_name: params.query_name,
            query_type: params.query_type,
            client_ip: params.client_ip,
            cache_hit: params.cache_hit,
            start_time: params.start_time.and_then(|t| {
                chrono::DateTime::parse_from_rfc3339(&t)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc))
            }),
            end_time: params.end_time.and_then(|t| {
                chrono::DateTime::parse_from_rfc3339(&t)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc))
            }),
            limit: params.limit,
            offset: params.offset,
        }
    }
}

/// Paginated logs response
#[derive(Debug, Serialize)]
pub struct LogsListResponse {
    pub data: Vec<QueryLog>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub has_more: bool,
}

impl From<PaginatedResult<QueryLog>> for LogsListResponse {
    fn from(result: PaginatedResult<QueryLog>) -> Self {
        let has_more = result.offset + (result.items.len() as i64) < result.total;
        Self {
            data: result.items,
            total: result.total,
            limit: result.limit,
            offset: result.offset,
            has_more,
        }
    }
}

/// Query statistics response
#[derive(Debug, Serialize)]
pub struct QueryStatsResponse {
    pub total_queries: i64,
    pub cache_hits: i64,
    pub queries_today: i64,
    pub cache_hit_rate: f64,
}

impl From<QueryStats> for QueryStatsResponse {
    fn from(stats: QueryStats) -> Self {
        let cache_hit_rate = if stats.total_queries > 0 {
            stats.cache_hits as f64 / stats.total_queries as f64
        } else {
            0.0
        };
        Self {
            total_queries: stats.total_queries,
            cache_hits: stats.cache_hits,
            queries_today: stats.queries_today,
            cache_hit_rate,
        }
    }
}

/// List query logs with pagination and filtering
///
/// GET /api/logs
/// List query logs with pagination and filtering
///
/// GET /api/logs
pub async fn list_logs(
    State(state): State<LogsState>,
    Query(params): Query<LogsQueryParams>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state.business.list(QueryLogFilter::from(params)).await?;
    Ok(Json(LogsListResponse::from(result)))
}

/// Get query statistics
///
/// GET /api/logs/stats
pub async fn get_stats(State(state): State<LogsState>) -> Result<impl IntoResponse, ApiError> {
    let stats = state.business.stats().await?;
    Ok(Json(QueryStatsResponse::from(stats)))
}

/// Cleanup parameters for retention-based deletion
#[derive(Debug, Clone, Deserialize)]
pub struct CleanupParams {
    #[serde(default = "default_retention_days")]
    pub days: i64,
}

fn default_retention_days() -> i64 {
    30
}

/// Delete query logs older than the given number of days
///
/// DELETE /api/logs/cleanup
pub async fn cleanup_logs(
    State(state): State<LogsState>,
    Query(params): Query<CleanupParams>,
) -> Result<impl IntoResponse, ApiError> {
    let deleted = state.business.cleanup_older_than(params.days).await?;

    Ok(Json(serde_json::json!({
        "deleted": deleted,
        "message": format!("Deleted {} query logs older than {} days", deleted, params.days)
    })))
}

/// Cleanup parameters for date-based deletion
#[derive(Debug, Clone, Deserialize)]
pub struct CleanupBeforeDateParams {
    pub before_date: String,
}

/// Delete query logs recorded before a specific date
///
/// DELETE /api/logs/cleanup-before
pub async fn cleanup_logs_before_date(
    State(state): State<LogsState>,
    Query(params): Query<CleanupBeforeDateParams>,
) -> Result<impl IntoResponse, ApiError> {
    let deleted = state
        .business
        .cleanup_before_date(&params.before_date)
        .await?;

    Ok(Json(serde_json::json!({
        "deleted": deleted,
        "message": format!("Deleted {} query logs before {}", deleted, params.before_date)
    })))
}

/// Delete every query log
///
/// DELETE /api/logs/cleanup-all
pub async fn cleanup_all_logs(
    State(state): State<LogsState>,
) -> Result<impl IntoResponse, ApiError> {
    let deleted = state.business.cleanup_all().await?;

    Ok(Json(serde_json::json!({
        "deleted": deleted,
        "message": format!("Deleted all {} query logs", deleted)
    })))
}

/// Export query logs as CSV or JSON
///
/// GET /api/logs/export
pub async fn export_logs(
    State(state): State<LogsState>,
    Query(params): Query<LogsQueryParams>,
) -> Result<impl IntoResponse, ApiError> {
    let format = params.format.clone().unwrap_or_else(|| "csv".to_string());
    let logs = state
        .business
        .list_for_export(QueryLogFilter::from(params), EXPORT_MAX_ROWS)
        .await?;

    if format.eq_ignore_ascii_case("json") {
        return Ok(Json(logs).into_response());
    }

    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "text/csv"),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"query_logs.csv\"",
            ),
        ],
        to_csv(&logs),
    )
        .into_response())
}

/// Render query logs as CSV, escaping fields that may contain separators.
fn to_csv(logs: &[QueryLog]) -> String {
    let mut csv = String::from(
        "Time,Client IP,Domain,Type,Response Code,Response Time(ms),Cache Hit,Upstream\n",
    );

    for log in logs {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            csv_field(&log.created_at.to_rfc3339()),
            csv_field(&log.client_ip),
            csv_field(&log.query_name),
            csv_field(&log.query_type),
            csv_field(log.response_code.as_deref().unwrap_or_default()),
            log.response_time.unwrap_or(0),
            log.cache_hit,
            csv_field(log.upstream_used.as_deref().unwrap_or_default()),
        ));
    }

    csv
}

/// Quote a CSV field when it contains a separator, quote or newline.
fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// Get log retention settings
///
/// GET /api/logs/retention
pub async fn get_retention_settings(
    State(state): State<LogsState>,
) -> Result<impl IntoResponse, ApiError> {
    let settings = state.business.retention_settings().await?;

    Ok(Json(serde_json::json!({
        "auto_cleanup_enabled": settings.auto_cleanup_enabled,
        "retention_days": settings.retention_days,
        "oldest_log_date": settings.oldest_log_date
    })))
}

/// Update log retention settings
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateRetentionParams {
    pub auto_cleanup_enabled: Option<bool>,
    pub retention_days: Option<i64>,
}

/// Update log retention settings
///
/// PUT /api/logs/retention
pub async fn update_retention_settings(
    State(state): State<LogsState>,
    Json(params): Json<UpdateRetentionParams>,
) -> Result<impl IntoResponse, ApiError> {
    let settings = state
        .business
        .update_retention_settings(params.auto_cleanup_enabled, params.retention_days)
        .await?;

    Ok(Json(serde_json::json!({
        "auto_cleanup_enabled": settings.auto_cleanup_enabled,
        "retention_days": settings.retention_days,
        "oldest_log_date": settings.oldest_log_date
    })))
}

/// Build the logs API router
pub fn logs_router(state: LogsState) -> axum::Router {
    use axum::routing::{delete, get};

    axum::Router::new()
        .route("/", get(list_logs))
        .route("/stats", get(get_stats))
        .route("/export", get(export_logs))
        .route("/cleanup", delete(cleanup_logs))
        .route("/cleanup-before", delete(cleanup_logs_before_date))
        .route("/cleanup-all", delete(cleanup_all_logs))
        .route(
            "/retention",
            get(get_retention_settings).put(update_retention_settings),
        )
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logs_query_params_to_filter() {
        let params = LogsQueryParams {
            query_name: Some("example.com".to_string()),
            query_type: Some("A".to_string()),
            client_ip: None,
            cache_hit: Some(true),
            start_time: None,
            end_time: None,
            limit: Some(50),
            offset: Some(0),
            format: None,
        };
        let filter = QueryLogFilter::from(params);
        assert_eq!(filter.query_name, Some("example.com".to_string()));
        assert_eq!(filter.query_type, Some("A".to_string()));
        assert_eq!(filter.cache_hit, Some(true));
        assert_eq!(filter.limit, Some(50));
        assert_eq!(filter.offset, Some(0));
    }

    #[test]
    fn test_query_stats_response_from() {
        let stats = QueryStats {
            total_queries: 1000,
            cache_hits: 750,
            queries_today: 100,
        };
        let response = QueryStatsResponse::from(stats);
        assert_eq!(response.total_queries, 1000);
        assert_eq!(response.cache_hits, 750);
        assert_eq!(response.queries_today, 100);
        assert!((response.cache_hit_rate - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_query_stats_response_zero_queries() {
        let stats = QueryStats {
            total_queries: 0,
            cache_hits: 0,
            queries_today: 0,
        };
        let response = QueryStatsResponse::from(stats);
        assert_eq!(response.cache_hit_rate, 0.0);
    }

    #[test]
    fn test_logs_list_response_has_more() {
        use chrono::Utc;

        // Create a dummy QueryLog for testing
        let dummy_log = QueryLog {
            id: 1,
            query_name: "example.com".to_string(),
            query_type: "A".to_string(),
            client_ip: "127.0.0.1".to_string(),
            response_code: Some("NOERROR".to_string()),
            response_time: Some(10),
            cache_hit: false,
            upstream_used: None,
            created_at: Utc::now(),
        };

        // Case 1: 50 items returned, total is 100, so has_more should be true
        let result = PaginatedResult {
            items: vec![dummy_log.clone(); 50],
            total: 100,
            limit: 50,
            offset: 0,
        };
        let response = LogsListResponse::from(result);
        assert!(response.has_more);

        // Case 2: 50 items returned, total is 50, so has_more should be false
        let result = PaginatedResult {
            items: vec![dummy_log; 50],
            total: 50,
            limit: 50,
            offset: 0,
        };
        let response = LogsListResponse::from(result);
        assert!(!response.has_more);
    }
}

//! Upstream Servers API module
//!
//! Implements REST API endpoints for upstream DNS server management.
//!
//! # Requirements
//!
//! - 4.4: Provide upstream server configuration functionality

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::db::{CreateUpstreamServer, Database, UpdateUpstreamServer, UpstreamServer};
use crate::dns::proxy::UpstreamManager;
use crate::web::ApiError;

/// Application state for upstream servers API
#[derive(Clone)]
pub struct UpstreamsState {
    pub db: Arc<Database>,
    pub upstream_manager: Arc<UpstreamManager>,
}

/// Valid protocol types
const VALID_PROTOCOLS: &[&str] = &["udp", "dot", "doh", "doq", "doh3"];

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

/// Create upstream server request with validation
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUpstreamServerRequest {
    pub name: String,
    pub address: String,
    pub protocol: String,
    #[serde(default = "default_timeout")]
    pub timeout: i32,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_timeout() -> i32 {
    5000
}

fn default_enabled() -> bool {
    true
}

/// Update upstream server request
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUpstreamServerRequest {
    pub name: Option<String>,
    pub address: Option<String>,
    pub protocol: Option<String>,
    pub timeout: Option<i32>,
    pub enabled: Option<bool>,
}

/// API response wrapper for single server
#[derive(Debug, Serialize)]
pub struct UpstreamServerResponse {
    pub data: UpstreamServer,
}

/// API response wrapper for multiple servers
#[derive(Debug, Serialize)]
pub struct UpstreamServersListResponse {
    pub data: Vec<UpstreamServer>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

/// Pagination query parameters
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    20
}

/// Server status information
#[derive(Debug, Serialize)]
pub struct ServerStatus {
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

/// API response for server status
#[derive(Debug, Serialize)]
pub struct ServerStatusResponse {
    pub data: Vec<ServerStatus>,
}

/// Validate server name
fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Name cannot be empty".to_string());
    }
    if name.len() > 100 {
        return Err("Name cannot exceed 100 characters".to_string());
    }
    Ok(())
}

/// Validate server address
fn validate_address(address: &str, protocol: &str) -> Result<(), String> {
    if address.is_empty() {
        return Err("Address cannot be empty".to_string());
    }
    if address.len() > 255 {
        return Err("Address cannot exceed 255 characters".to_string());
    }

    match protocol.to_lowercase().as_str() {
        "udp" | "dot" | "doq" => {
            // Should be host:port format or just IP
            // Basic validation - check if it looks like a valid address
            if !address.contains(':') && !address.contains('.') {
                return Err("Address should be in host:port format (e.g., 8.8.8.8:53)".to_string());
            }
        }
        "doh" | "doh3" => {
            // Should be a URL
            if !address.starts_with("https://") && !address.starts_with("http://") {
                return Err("DoH/DoH3 address should be a URL (e.g., https://dns.google/dns-query)".to_string());
            }
        }
        _ => {}
    }
    Ok(())
}

/// Validate protocol
fn validate_protocol(protocol: &str) -> Result<(), String> {
    let lower = protocol.to_lowercase();
    if !VALID_PROTOCOLS.contains(&lower.as_str()) {
        return Err(format!(
            "Invalid protocol. Must be one of: {}",
            VALID_PROTOCOLS.join(", ")
        ));
    }
    Ok(())
}

/// Validate timeout
fn validate_timeout(timeout: i32) -> Result<(), String> {
    if timeout < 100 {
        return Err("Timeout must be at least 100ms".to_string());
    }
    if timeout > 60000 {
        return Err("Timeout cannot exceed 60000ms (60 seconds)".to_string());
    }
    Ok(())
}

impl CreateUpstreamServerRequest {
    /// Validate the create request
    pub fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = Vec::new();

        if let Err(e) = validate_name(&self.name) {
            errors.push(ValidationError {
                field: "name".to_string(),
                message: e,
            });
        }

        if let Err(e) = validate_protocol(&self.protocol) {
            errors.push(ValidationError {
                field: "protocol".to_string(),
                message: e,
            });
        }

        // Only validate address if protocol is valid
        if validate_protocol(&self.protocol).is_ok() {
            if let Err(e) = validate_address(&self.address, &self.protocol) {
                errors.push(ValidationError {
                    field: "address".to_string(),
                    message: e,
                });
            }
        } else if self.address.is_empty() {
            errors.push(ValidationError {
                field: "address".to_string(),
                message: "Address cannot be empty".to_string(),
            });
        }

        if let Err(e) = validate_timeout(self.timeout) {
            errors.push(ValidationError {
                field: "timeout".to_string(),
                message: e,
            });
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationErrors { errors })
        }
    }

    /// Convert to CreateUpstreamServer with normalized values
    pub fn into_create_upstream_server(self) -> CreateUpstreamServer {
        CreateUpstreamServer {
            name: self.name,
            address: self.address,
            protocol: self.protocol.to_lowercase(),
            timeout: self.timeout,
            enabled: self.enabled,
        }
    }
}

impl UpdateUpstreamServerRequest {
    /// Validate the update request
    pub fn validate(&self, existing: &UpstreamServer) -> Result<(), ValidationErrors> {
        let mut errors = Vec::new();

        if let Some(ref name) = self.name {
            if let Err(e) = validate_name(name) {
                errors.push(ValidationError {
                    field: "name".to_string(),
                    message: e,
                });
            }
        }

        if let Some(ref protocol) = self.protocol {
            if let Err(e) = validate_protocol(protocol) {
                errors.push(ValidationError {
                    field: "protocol".to_string(),
                    message: e,
                });
            }
        }

        // Validate address against protocol (new or existing)
        if let Some(ref address) = self.address {
            let protocol = self.protocol.as_deref().unwrap_or(&existing.protocol);
            if validate_protocol(protocol).is_ok() {
                if let Err(e) = validate_address(address, protocol) {
                    errors.push(ValidationError {
                        field: "address".to_string(),
                        message: e,
                    });
                }
            }
        }

        if let Some(timeout) = self.timeout {
            if let Err(e) = validate_timeout(timeout) {
                errors.push(ValidationError {
                    field: "timeout".to_string(),
                    message: e,
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationErrors { errors })
        }
    }

    /// Convert to UpdateUpstreamServer with normalized values
    pub fn into_update_upstream_server(self) -> UpdateUpstreamServer {
        UpdateUpstreamServer {
            name: self.name,
            address: self.address,
            protocol: self.protocol.map(|p| p.to_lowercase()),
            timeout: self.timeout,
            enabled: self.enabled,
        }
    }
}

/// List all upstream servers with pagination
///
/// GET /api/upstreams?page=1&page_size=20
pub async fn list_upstreams(
    State(state): State<UpstreamsState>,
    axum::extract::Query(pagination): axum::extract::Query<PaginationQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = state.db.upstream_servers();

    // Validate pagination parameters
    let page = pagination.page.max(1);
    let page_size = pagination.page_size.clamp(1, 100);

    let (servers, total) = repo.list_paged(page, page_size).await.map_err(|e| ApiError {
        code: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to list upstream servers: {}", e),
        details: None,
    })?;

    Ok(Json(UpstreamServersListResponse {
        total,
        page,
        page_size,
        data: servers,
    }))
}

/// Get an upstream server by ID
///
/// GET /api/upstreams/:id
pub async fn get_upstream(
    State(state): State<UpstreamsState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = state.db.upstream_servers();

    let server = repo.get_by_id(id).await.map_err(|e| ApiError {
        code: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to get upstream server: {}", e),
        details: None,
    })?;

    match server {
        Some(s) => Ok(Json(UpstreamServerResponse { data: s })),
        None => Err(ApiError {
            code: "NOT_FOUND".to_string(),
            message: format!("Upstream server with id {} not found", id),
            details: None,
        }),
    }
}

/// Create a new upstream server
///
/// POST /api/upstreams
pub async fn create_upstream(
    State(state): State<UpstreamsState>,
    Json(request): Json<CreateUpstreamServerRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate request
    if let Err(validation_errors) = request.validate() {
        return Err(ApiError {
            code: "BAD_REQUEST".to_string(),
            message: "Validation failed".to_string(),
            details: Some(serde_json::to_value(validation_errors).unwrap()),
        });
    }

    let repo = state.db.upstream_servers();
    let create_server = request.into_create_upstream_server();

    let server = repo.create(create_server).await.map_err(|e| ApiError {
        code: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to create upstream server: {}", e),
        details: None,
    })?;

    // Reload upstream servers in the manager
    if let Err(e) = state.upstream_manager.reload_from_db(&state.db).await {
        tracing::warn!("Failed to reload upstream servers: {}", e);
    }

    Ok((StatusCode::CREATED, Json(UpstreamServerResponse { data: server })))
}

/// Update an upstream server
///
/// PUT /api/upstreams/:id
pub async fn update_upstream(
    State(state): State<UpstreamsState>,
    Path(id): Path<i64>,
    Json(request): Json<UpdateUpstreamServerRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = state.db.upstream_servers();

    // First check if server exists
    let existing = repo.get_by_id(id).await.map_err(|e| ApiError {
        code: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to get upstream server: {}", e),
        details: None,
    })?;

    let existing = existing.ok_or_else(|| ApiError {
        code: "NOT_FOUND".to_string(),
        message: format!("Upstream server with id {} not found", id),
        details: None,
    })?;

    // Validate request against existing server
    if let Err(validation_errors) = request.validate(&existing) {
        return Err(ApiError {
            code: "BAD_REQUEST".to_string(),
            message: "Validation failed".to_string(),
            details: Some(serde_json::to_value(validation_errors).unwrap()),
        });
    }

    let update_server = request.into_update_upstream_server();

    let server = repo.update(id, update_server).await.map_err(|e| ApiError {
        code: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to update upstream server: {}", e),
        details: None,
    })?;

    // Reload upstream servers in the manager
    if let Err(e) = state.upstream_manager.reload_from_db(&state.db).await {
        tracing::warn!("Failed to reload upstream servers: {}", e);
    }

    match server {
        Some(s) => Ok(Json(UpstreamServerResponse { data: s })),
        None => Err(ApiError {
            code: "NOT_FOUND".to_string(),
            message: format!("Upstream server with id {} not found", id),
            details: None,
        }),
    }
}

/// Delete an upstream server
///
/// DELETE /api/upstreams/:id
pub async fn delete_upstream(
    State(state): State<UpstreamsState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = state.db.upstream_servers();

    let deleted = repo.delete(id).await.map_err(|e| ApiError {
        code: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to delete upstream server: {}", e),
        details: None,
    })?;

    if deleted {
        // Reload upstream servers in the manager
        if let Err(e) = state.upstream_manager.reload_from_db(&state.db).await {
            tracing::warn!("Failed to reload upstream servers: {}", e);
        }
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError {
            code: "NOT_FOUND".to_string(),
            message: format!("Upstream server with id {} not found", id),
            details: None,
        })
    }
}

/// Get upstream server status
///
/// GET /api/upstreams/status
pub async fn get_status(
    State(state): State<UpstreamsState>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = state.db.upstream_servers();

    let servers = repo.list().await.map_err(|e| ApiError {
        code: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to list upstream servers: {}", e),
        details: None,
    })?;

    let stats = state.upstream_manager.get_all_stats().await;

    let status: Vec<ServerStatus> = servers
        .into_iter()
        .map(|s| {
            let server_stats = stats.get(&s.id);
            ServerStatus {
                id: s.id,
                name: s.name,
                address: s.address,
                protocol: s.protocol,
                enabled: s.enabled,
                healthy: server_stats.map(|st| st.is_healthy()).unwrap_or(s.enabled),
                queries: server_stats.map(|st| st.queries).unwrap_or(0),
                failures: server_stats.map(|st| st.failures).unwrap_or(0),
                avg_response_time_ms: server_stats.map(|st| st.avg_response_time_ms()).unwrap_or(0),
            }
        })
        .collect();

    Ok(Json(ServerStatusResponse { data: status }))
}

/// Build the upstream servers API router
pub fn upstreams_router(state: UpstreamsState) -> axum::Router {
    use axum::routing::get;

    axum::Router::new()
        .route("/", get(list_upstreams).post(create_upstream))
        .route("/status", get(get_status))
        .route("/:id", get(get_upstream).put(update_upstream).delete(delete_upstream))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_name_valid() {
        assert!(validate_name("Cloudflare").is_ok());
        assert!(validate_name("Google DNS").is_ok());
        assert!(validate_name("My-Server_1").is_ok());
    }

    #[test]
    fn test_validate_name_invalid() {
        assert!(validate_name("").is_err());
        assert!(validate_name(&"a".repeat(101)).is_err());
    }

    #[test]
    fn test_validate_protocol_valid() {
        assert!(validate_protocol("udp").is_ok());
        assert!(validate_protocol("UDP").is_ok());
        assert!(validate_protocol("dot").is_ok());
        assert!(validate_protocol("doh").is_ok());
        assert!(validate_protocol("doq").is_ok());
    }

    #[test]
    fn test_validate_protocol_invalid() {
        assert!(validate_protocol("invalid").is_err());
        assert!(validate_protocol("").is_err());
    }

    #[test]
    fn test_validate_address_udp() {
        assert!(validate_address("8.8.8.8:53", "udp").is_ok());
        assert!(validate_address("1.1.1.1:53", "udp").is_ok());
    }

    #[test]
    fn test_validate_address_doh() {
        assert!(validate_address("https://dns.google/dns-query", "doh").is_ok());
        assert!(validate_address("dns.google", "doh").is_err());
    }

    #[test]
    fn test_validate_timeout() {
        assert!(validate_timeout(100).is_ok());
        assert!(validate_timeout(5000).is_ok());
        assert!(validate_timeout(60000).is_ok());
        assert!(validate_timeout(99).is_err());
        assert!(validate_timeout(60001).is_err());
    }

    #[test]
    fn test_create_request_validation() {
        let valid_request = CreateUpstreamServerRequest {
            name: "Cloudflare".to_string(),
            address: "1.1.1.1:53".to_string(),
            protocol: "udp".to_string(),
            timeout: 5000,
            enabled: true,
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = CreateUpstreamServerRequest {
            name: "".to_string(),
            address: "".to_string(),
            protocol: "invalid".to_string(),
            timeout: 50,
            enabled: true,
        };
        let result = invalid_request.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_create_request_into_create_upstream_server() {
        let request = CreateUpstreamServerRequest {
            name: "Test".to_string(),
            address: "8.8.8.8:53".to_string(),
            protocol: "UDP".to_string(),
            timeout: 5000,
            enabled: true,
        };
        let create_server = request.into_create_upstream_server();
        assert_eq!(create_server.protocol, "udp");
    }
}

//! Server Listeners API
//!
//! API endpoints for managing DNS server listeners (UDP, DoT, DoH, DoQ, DoH3).

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use super::ApiError;
use crate::business::listener_business::ListenerBusiness;
use crate::infrastructure::common::AppError;
use crate::infrastructure::repository::{ServerListener, UpdateServerListener};

/// Listeners API state
#[derive(Clone)]
pub struct ListenersState {
    pub business: Arc<ListenerBusiness>,
}

/// Listener response
#[derive(Debug, Serialize)]
pub struct ListenerResponse {
    pub protocol: String,
    pub enabled: bool,
    pub bind_address: String,
    pub port: i32,
    pub has_tls_cert: bool,
    pub has_tls_key: bool,
    pub requires_tls: bool,
    pub description: String,
    pub tls_cert: Option<String>,
    pub tls_key: Option<String>,
}

impl From<ServerListener> for ListenerResponse {
    fn from(l: ServerListener) -> Self {
        let (requires_tls, description) = match l.protocol.as_str() {
            "udp" => (false, "标准 UDP DNS (端口 53)".to_string()),
            "dot" => (true, "DNS over TLS (端口 853)".to_string()),
            "doh" => (true, "DNS over HTTPS (端口 443)".to_string()),
            "doq" => (true, "DNS over QUIC (端口 853)".to_string()),
            "doh3" => (true, "DNS over HTTP/3 (端口 443)".to_string()),
            _ => (false, "未知协议".to_string()),
        };

        Self {
            protocol: l.protocol,
            enabled: l.enabled,
            bind_address: l.bind_address,
            port: l.port,
            has_tls_cert: l.tls_cert.is_some(),
            has_tls_key: l.tls_key.is_some(),
            requires_tls,
            description,
            tls_cert: l.tls_cert,
            tls_key: l.tls_key,
        }
    }
}

/// List listeners response
#[derive(Debug, Serialize)]
pub struct ListListenersResponse {
    pub data: Vec<ListenerResponse>,
}

/// Update listener request
#[derive(Debug, Deserialize)]
pub struct UpdateListenerRequest {
    pub enabled: Option<bool>,
    pub bind_address: Option<String>,
    pub port: Option<i32>,
    pub tls_cert: Option<String>,
    pub tls_key: Option<String>,
}

impl UpdateListenerRequest {
    /// Validate listener input at the HTTP boundary.
    fn validate(&self) -> Result<(), String> {
        if let Some(port) = self.port {
            if !(1..=65535).contains(&port) {
                return Err("端口必须在 1-65535 之间".to_string());
            }
        }
        if let Some(cert) = self.tls_cert.as_deref() {
            if !cert.trim().is_empty() && !cert.contains("-----BEGIN CERTIFICATE-----") {
                return Err("证书格式无效，请提供 PEM 格式的证书".to_string());
            }
        }
        if let Some(key) = self.tls_key.as_deref() {
            if !key.trim().is_empty() && !key.contains("-----BEGIN") {
                return Err("私钥格式无效，请提供 PEM 格式的私钥".to_string());
            }
        }
        Ok(())
    }
}

/// Certificate information response
#[derive(Debug, Serialize)]
pub struct CertificateInfo {
    pub subject: String,
    pub issuer: String,
    pub not_before: String,
    pub not_after: String,
    pub serial_number: String,
    pub is_expired: bool,
    pub days_until_expiry: i64,
}

/// Create the listeners router
/// Create the listeners router
pub fn listeners_router(state: ListenersState) -> Router {
    Router::new()
        .route("/", get(list_listeners))
        .route("/:protocol", get(get_listener).put(update_listener))
        .route("/:protocol/cert", get(get_certificate_info))
        .with_state(state)
}

/// List all server listeners
async fn list_listeners(
    State(state): State<ListenersState>,
) -> Result<impl IntoResponse, ApiError> {
    let listeners = state.business.list().await?;
    let data: Vec<ListenerResponse> = listeners.into_iter().map(Into::into).collect();
    Ok(Json(ListListenersResponse { data }))
}

/// Get a specific listener by protocol
async fn get_listener(
    State(state): State<ListenersState>,
    Path(protocol): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let listener = state.business.get(&protocol).await?;
    Ok(Json(ListenerResponse::from(listener)))
}

/// Update a listener and apply the change to the running process
async fn update_listener(
    State(state): State<ListenersState>,
    Path(protocol): Path<String>,
    Json(request): Json<UpdateListenerRequest>,
) -> Result<impl IntoResponse, ApiError> {
    request.validate().map_err(AppError::Validation)?;

    let listener = state
        .business
        .update(
            &protocol,
            UpdateServerListener {
                enabled: request.enabled,
                bind_address: request.bind_address,
                port: request.port,
                // Empty strings are preserved so the repository can clear the field.
                tls_cert: request.tls_cert.map(|s| s.trim().to_string()),
                tls_key: request.tls_key.map(|s| s.trim().to_string()),
            },
        )
        .await?;

    Ok((StatusCode::OK, Json(ListenerResponse::from(listener))))
}

/// Get certificate information for a listener
async fn get_certificate_info(
    State(state): State<ListenersState>,
    Path(protocol): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let details = state.business.certificate_details(&protocol).await?;

    Ok(Json(CertificateInfo {
        subject: details.subject,
        issuer: details.issuer,
        not_before: details.not_before,
        not_after: details.not_after,
        serial_number: details.serial_number,
        is_expired: details.is_expired,
        days_until_expiry: details.days_until_expiry,
    }))
}

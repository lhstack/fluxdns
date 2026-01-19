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

use crate::db::{Database, ServerListener, UpdateServerListener};
use super::ApiError;

use crate::services::listener_manager::ListenerManager;

/// Listeners API state
#[derive(Clone)]
pub struct ListenersState {
    pub db: Arc<Database>,
    pub listener_manager: Arc<ListenerManager>,
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
    let listeners = state.db.server_listeners().list().await.map_err(|e| ApiError {
        code: "DATABASE_ERROR".to_string(),
        message: format!("Failed to list listeners: {}", e),
        details: None,
    })?;

    let response: Vec<ListenerResponse> = listeners.into_iter().map(|l| l.into()).collect();

    Ok(Json(ListListenersResponse { data: response }))
}

/// Get a specific listener by protocol
async fn get_listener(
    State(state): State<ListenersState>,
    Path(protocol): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let listener = state.db.server_listeners().get_by_protocol(&protocol).await.map_err(|e| ApiError {
        code: "DATABASE_ERROR".to_string(),
        message: format!("Failed to get listener: {}", e),
        details: None,
    })?;

    match listener {
        Some(l) => Ok(Json(ListenerResponse::from(l))),
        None => Err(ApiError {
            code: "NOT_FOUND".to_string(),
            message: format!("Listener '{}' not found", protocol),
            details: None,
        }),
    }
}

/// Update a listener
async fn update_listener(
    State(state): State<ListenersState>,
    Path(protocol): Path<String>,
    Json(request): Json<UpdateListenerRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate port
    if let Some(port) = request.port {
        if port < 1 || port > 65535 {
            return Err(ApiError {
                code: "VALIDATION_ERROR".to_string(),
                message: "端口必须在 1-65535 之间".to_string(),
                details: None,
            });
        }
    }

    // Validate TLS cert format if provided
    if let Some(ref cert) = request.tls_cert {
        if !cert.trim().is_empty() && !cert.contains("-----BEGIN CERTIFICATE-----") {
            return Err(ApiError {
                code: "VALIDATION_ERROR".to_string(),
                message: "证书格式无效，请提供 PEM 格式的证书".to_string(),
                details: None,
            });
        }
    }

    // Validate TLS key format if provided
    if let Some(ref key) = request.tls_key {
        if !key.trim().is_empty() && !key.contains("-----BEGIN") {
            return Err(ApiError {
                code: "VALIDATION_ERROR".to_string(),
                message: "私钥格式无效，请提供 PEM 格式的私钥".to_string(),
                details: None,
            });
        }
    }

    let update = UpdateServerListener {
        enabled: request.enabled,
        bind_address: request.bind_address,
        port: request.port,
        // Don't flatten/filter empty strings here. Passes Some("") to repository to indicate truncation.
        tls_cert: request.tls_cert.map(|s| s.trim().to_string()),
        tls_key: request.tls_key.map(|s| s.trim().to_string()),
    };

    let listener = state.db.server_listeners().update(&protocol, update).await.map_err(|e| ApiError {
        code: "DATABASE_ERROR".to_string(),
        message: format!("更新失败: {}", e),
        details: None,
    })?;

    match listener {
        Some(l) => {
            // Manage lifecycle via ListenerManager
            if l.enabled {
                if let Err(e) = state.listener_manager.start_listener(&protocol).await {
                    tracing::error!("Failed to start {} listener: {}", protocol, e);
                    
                    // Revert database status to disabled
                    let revert = UpdateServerListener {
                        enabled: Some(false),
                        ..Default::default()
                    };
                    let _ = state.db.server_listeners().update(&protocol, revert).await;
                    
                    return Err(ApiError {
                        code: "START_FAILED".into(),
                        message: format!("启动失败: {}", e),
                        details: None,
                    });
                }
            } else {
                state.listener_manager.stop_listener(&protocol).await;
            }

            // Warn if TLS protocol is enabled without certificates
            let requires_tls = matches!(protocol.as_str(), "dot" | "doh" | "doq" | "doh3");
            if l.enabled && requires_tls && (l.tls_cert.is_none() || l.tls_key.is_none()) {
                tracing::warn!(
                    "Listener {} enabled but TLS certificates not configured.",
                    protocol
                );
            }
            tracing::info!("Listener {} updated: enabled={}, port={}", protocol, l.enabled, l.port);
            Ok((StatusCode::OK, Json(ListenerResponse::from(l))))
        }
        None => Err(ApiError {
            code: "NOT_FOUND".to_string(),
            message: format!("监听器 '{}' 不存在", protocol),
            details: None,
        }),
    }
}

/// Get certificate information for a listener
async fn get_certificate_info(
    State(state): State<ListenersState>,
    Path(protocol): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let listener = state.db.server_listeners().get_by_protocol(&protocol).await.map_err(|e| ApiError {
        code: "DATABASE_ERROR".to_string(),
        message: format!("Failed to get listener: {}", e),
        details: None,
    })?;

    let listener = match listener {
        Some(l) => l,
        None => return Err(ApiError {
            code: "NOT_FOUND".to_string(),
            message: format!("Listener '{}' not found", protocol),
            details: None,
        }),
    };

    let cert_pem = match listener.tls_cert {
        Some(c) => c,
        None => return Err(ApiError {
            code: "NO_CERTIFICATE".to_string(),
            message: "该监听器未配置证书".to_string(),
            details: None,
        }),
    };

    // Parse PEM to DER
    let certs: Vec<_> = rustls_pemfile::certs(&mut cert_pem.as_bytes())
        .filter_map(|r| r.ok())
        .collect();

    if certs.is_empty() {
        return Err(ApiError {
            code: "INVALID_CERTIFICATE".to_string(),
            message: "无法解析证书内容".to_string(),
            details: None,
        });
    }

    // Parse X.509
    use x509_parser::prelude::*;
    let (_, cert) = X509Certificate::from_der(&certs[0]).map_err(|e| ApiError {
        code: "PARSE_ERROR".to_string(),
        message: format!("证书解析失败: {}", e),
        details: None,
    })?;

    let now = chrono::Utc::now();
    let not_before = cert.validity().not_before.to_datetime();
    let not_after = cert.validity().not_after.to_datetime();
    
    // Convert time::OffsetDateTime to chrono::DateTime via unix timestamp
    let not_after_chrono = chrono::DateTime::from_timestamp(not_after.unix_timestamp(), 0)
        .unwrap_or(now);
    
    let is_expired = now > not_after_chrono;
    let days_until_expiry = (not_after_chrono - now).num_days();

    let info = CertificateInfo {
        subject: cert.subject().to_string(),
        issuer: cert.issuer().to_string(),
        not_before: not_before.to_string(),
        not_after: not_after.to_string(),
        serial_number: cert.serial.to_str_radix(16),
        is_expired,
        days_until_expiry,
    };

    Ok(Json(info))
}

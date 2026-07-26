//! Settings API module
//!
//! Implements REST API endpoints for system settings management.

use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

use crate::application::web::ApiError;
use crate::business::setting_business::{SettingBusiness, SettingsUpdate};

/// Application state for settings API
#[derive(Clone)]
pub struct SettingsState {
    pub business: Arc<SettingBusiness>,
}

/// System settings response
#[derive(Debug, Serialize)]
pub struct SystemSettings {
    /// Disabled record types (e.g., ["AAAA"] to disable IPv6)
    pub disabled_record_types: Vec<String>,
    /// Alert settings
    pub alert_enabled: bool,
    pub alert_webhook_url: Option<String>,
    pub alert_latency_threshold_ms: i64,
}

impl From<crate::business::setting_business::SystemSettings> for SystemSettings {
    fn from(s: crate::business::setting_business::SystemSettings) -> Self {
        Self {
            disabled_record_types: s.disabled_record_types,
            alert_enabled: s.alert_enabled,
            alert_webhook_url: s.alert_webhook_url,
            alert_latency_threshold_ms: s.alert_latency_threshold_ms,
        }
    }
}

/// Update settings request
#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    /// Disabled record types
    pub disabled_record_types: Option<Vec<String>>,
    /// Alert settings
    pub alert_enabled: Option<bool>,
    pub alert_webhook_url: Option<String>,
    pub alert_latency_threshold_ms: Option<i64>,
}

/// Get current system settings
///
/// GET /api/settings
/// Get current system settings
///
/// GET /api/settings
pub async fn get_settings(
    State(state): State<SettingsState>,
) -> Result<impl IntoResponse, ApiError> {
    let settings = state.business.get().await?;
    Ok(Json(SystemSettings::from(settings)))
}

/// Update system settings
///
/// PUT /api/settings
pub async fn update_settings(
    State(state): State<SettingsState>,
    Json(request): Json<UpdateSettingsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let settings = state
        .business
        .update(SettingsUpdate {
            disabled_record_types: request.disabled_record_types,
            alert_enabled: request.alert_enabled,
            alert_webhook_url: request.alert_webhook_url,
            alert_latency_threshold_ms: request.alert_latency_threshold_ms,
        })
        .await?;

    Ok(Json(SystemSettings::from(settings)))
}

/// Send a test alert to the configured webhook
///
/// POST /api/settings/test-alert
pub async fn test_alert(State(state): State<SettingsState>) -> Result<impl IntoResponse, ApiError> {
    state.business.send_test_alert().await?;
    Ok(Json(serde_json::json!({ "status": "ok" })))
}

/// Build the settings API router
pub fn settings_router(state: SettingsState) -> axum::Router {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/", get(get_settings).put(update_settings))
        .route("/test-alert", post(test_alert))
        .with_state(state)
}

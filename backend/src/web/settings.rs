//! Settings API module
//!
//! Implements REST API endpoints for system settings management.

use std::sync::Arc;

use axum::{
    extract::State,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::web::ApiError;

/// Application state for settings API
#[derive(Clone)]
pub struct SettingsState {
    pub db: Arc<Database>,
}

/// System settings response
#[derive(Debug, Serialize)]
pub struct SystemSettings {
    /// Disabled record types (e.g., ["AAAA"] to disable IPv6)
    pub disabled_record_types: Vec<String>,
}

/// Update settings request
#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    /// Disabled record types
    pub disabled_record_types: Option<Vec<String>>,
}

/// Config key for disabled record types
const CONFIG_KEY_DISABLED_RECORD_TYPES: &str = "disabled_record_types";

/// Get current system settings
///
/// GET /api/settings
pub async fn get_settings(
    State(state): State<SettingsState>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = state.db.system_config();

    let disabled_record_types = repo.get(CONFIG_KEY_DISABLED_RECORD_TYPES).await
        .map_err(|e| ApiError {
            code: "INTERNAL_ERROR".to_string(),
            message: format!("Failed to get settings: {}", e),
            details: None,
        })?
        .map(|v| serde_json::from_str::<Vec<String>>(&v).unwrap_or_default())
        .unwrap_or_default();

    Ok(Json(SystemSettings {
        disabled_record_types,
    }))
}

/// Update system settings
///
/// PUT /api/settings
pub async fn update_settings(
    State(state): State<SettingsState>,
    Json(request): Json<UpdateSettingsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = state.db.system_config();

    if let Some(disabled_types) = request.disabled_record_types {
        // Validate record types
        let valid_types = ["A", "AAAA", "CNAME", "MX", "TXT", "PTR", "NS", "SOA", "SRV"];
        for t in &disabled_types {
            let upper = t.to_uppercase();
            if !valid_types.contains(&upper.as_str()) {
                return Err(ApiError {
                    code: "BAD_REQUEST".to_string(),
                    message: format!("Invalid record type: {}", t),
                    details: None,
                });
            }
        }

        let value = serde_json::to_string(&disabled_types).map_err(|e| ApiError {
            code: "INTERNAL_ERROR".to_string(),
            message: format!("Failed to serialize settings: {}", e),
            details: None,
        })?;

        repo.set(CONFIG_KEY_DISABLED_RECORD_TYPES, &value).await.map_err(|e| ApiError {
            code: "INTERNAL_ERROR".to_string(),
            message: format!("Failed to save settings: {}", e),
            details: None,
        })?;
    }

    // Return updated settings
    get_settings(State(state)).await
}

/// Build the settings API router
pub fn settings_router(state: SettingsState) -> axum::Router {
    use axum::routing::get;

    axum::Router::new()
        .route("/", get(get_settings).put(update_settings))
        .with_state(state)
}

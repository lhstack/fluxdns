// LLM Web API - REST endpoints for LLM configuration and chat

use axum::{
    extract::State,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::llm::{
    config::{LlmConfig, LlmConfigRequest},
    types::{get_provider_presets, ChatMessage, ProviderPreset, Role},
    FunctionRegistry, LlmClient,
};
use crate::state::AppState;
use crate::web::auth::ApiError;

/// LLM API State
#[derive(Clone)]
pub struct LlmState {
    pub app_state: Arc<AppState>,
}

/// Create LLM router
pub fn llm_router() -> Router<LlmState> {
    Router::new()
        // Configuration endpoints
        .route("/config", get(get_configs))
        .route("/config", post(create_config))
        .route("/config/:id", put(update_config))
        .route("/config/:id", delete(delete_config))
        .route("/config/:id/enable", post(enable_config))
        .route("/config/test", post(test_connection))
        // Provider presets
        .route("/providers", get(get_providers))
        // Chat endpoints
        .route("/chat", post(chat))
        .route("/conversations", get(get_conversations))
        .route("/conversations", delete(clear_conversations))
}

/// Helper to create internal error
fn internal_error(msg: impl ToString) -> ApiError {
    ApiError {
        code: "INTERNAL_ERROR".to_string(),
        message: msg.to_string(),
        details: None,
    }
}

/// Helper to create not found error
fn not_found(msg: impl ToString) -> ApiError {
    ApiError {
        code: "NOT_FOUND".to_string(),
        message: msg.to_string(),
        details: None,
    }
}

/// Helper to create bad request error
fn bad_request(msg: impl ToString) -> ApiError {
    ApiError {
        code: "BAD_REQUEST".to_string(),
        message: msg.to_string(),
        details: None,
    }
}

/// Get all LLM configurations
async fn get_configs(
    State(state): State<LlmState>,
) -> Result<Json<Vec<LlmConfigResponse>>, ApiError> {
    let configs = sqlx::query_as::<_, (i64, String, String, String, String, String, bool)>(
        "SELECT id, provider, display_name, api_base_url, api_key, model, enabled FROM llm_config"
    )
    .fetch_all(state.app_state.db.pool())
    .await
    .map_err(|e| internal_error(e))?;

    let response: Vec<LlmConfigResponse> = configs
        .into_iter()
        .map(|(id, provider, display_name, api_base_url, api_key, model, enabled)| {
            LlmConfigResponse {
                id,
                provider,
                display_name,
                api_base_url,
                api_key: api_key.clone(),
                api_key_masked: mask_api_key(&api_key),
                model,
                enabled,
            }
        })
        .collect();

    Ok(Json(response))
}

/// Create a new LLM configuration
async fn create_config(
    State(state): State<LlmState>,
    Json(req): Json<LlmConfigRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let display_name = req.display_name.unwrap_or_else(|| req.provider.clone());
    
    sqlx::query(
        "INSERT INTO llm_config (provider, display_name, api_base_url, api_key, model, enabled) VALUES (?, ?, ?, ?, ?, 0)"
    )
    .bind(&req.provider)
    .bind(&display_name)
    .bind(&req.api_base_url)
    .bind(&req.api_key)
    .bind(&req.model)
    .execute(state.app_state.db.pool())
    .await
    .map_err(|e| internal_error(e))?;

    Ok(Json(serde_json::json!({"success": true, "message": "配置已创建"})))
}

/// Update an existing configuration
async fn update_config(
    State(state): State<LlmState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(req): Json<LlmConfigRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let display_name = req.display_name.unwrap_or_else(|| req.provider.clone());
    
    let result = if req.api_key.is_empty() {
        // If API key is empty, don't update it
        sqlx::query(
            "UPDATE llm_config SET provider = ?, display_name = ?, api_base_url = ?, model = ? WHERE id = ?"
        )
        .bind(&req.provider)
        .bind(&display_name)
        .bind(&req.api_base_url)
        .bind(&req.model)
        .bind(id)
        .execute(state.app_state.db.pool())
        .await
    } else {
        sqlx::query(
            "UPDATE llm_config SET provider = ?, display_name = ?, api_base_url = ?, api_key = ?, model = ? WHERE id = ?"
        )
        .bind(&req.provider)
        .bind(&display_name)
        .bind(&req.api_base_url)
        .bind(&req.api_key)
        .bind(&req.model)
        .bind(id)
        .execute(state.app_state.db.pool())
        .await
    };

    let result = result.map_err(|e| internal_error(e))?;

    if result.rows_affected() == 0 {
        return Err(not_found("配置不存在"));
    }

    Ok(Json(serde_json::json!({"success": true})))
}

/// Delete a configuration
async fn delete_config(
    State(state): State<LlmState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query("DELETE FROM llm_config WHERE id = ?")
        .bind(id)
        .execute(state.app_state.db.pool())
        .await
        .map_err(|e| internal_error(e))?;

    if result.rows_affected() == 0 {
        return Err(not_found("配置不存在"));
    }

    Ok(Json(serde_json::json!({"success": true})))
}

/// Enable a specific configuration (disables others)
async fn enable_config(
    State(state): State<LlmState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Disable all configs first
    sqlx::query("UPDATE llm_config SET enabled = 0")
        .execute(state.app_state.db.pool())
        .await
        .map_err(|e| internal_error(e))?;

    // Enable the selected one
    let result = sqlx::query("UPDATE llm_config SET enabled = 1 WHERE id = ?")
        .bind(id)
        .execute(state.app_state.db.pool())
        .await
        .map_err(|e| internal_error(e))?;

    if result.rows_affected() == 0 {
        return Err(not_found("配置不存在"));
    }

    Ok(Json(serde_json::json!({"success": true, "message": "已启用"})))
}

/// Test LLM connection
async fn test_connection(
    State(state): State<LlmState>,
    Json(req): Json<LlmConfigRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let config = LlmConfig {
        id: 0,
        provider: req.provider,
        display_name: req.display_name.unwrap_or_default(),
        api_base_url: req.api_base_url,
        api_key: req.api_key,
        model: req.model,
        enabled: true,
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
    };

    let registry = Arc::new(FunctionRegistry::new(state.app_state.clone()));
    let client = LlmClient::new(config, registry);
    
    match client.test_connection().await {
        Ok(true) => Ok(Json(serde_json::json!({"success": true, "message": "连接成功"}))),
        Ok(false) => Ok(Json(serde_json::json!({"success": false, "message": "连接失败"}))),
        Err(e) => Ok(Json(serde_json::json!({"success": false, "message": e.to_string()}))),
    }
}

/// Get provider presets
async fn get_providers() -> Json<Vec<ProviderPreset>> {
    Json(get_provider_presets())
}

/// Chat request
#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub context: Option<String>,
}

/// Chat response
#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub reply: String,
    pub functions_called: Vec<String>,
}

/// Send a chat message
async fn chat(
    State(state): State<LlmState>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, ApiError> {
    // Get enabled config
    let config = sqlx::query_as::<_, (i64, String, String, String, String, String, bool, chrono::NaiveDateTime, chrono::NaiveDateTime)>(
        "SELECT id, provider, display_name, api_base_url, api_key, model, enabled, created_at, updated_at FROM llm_config WHERE enabled = 1 LIMIT 1"
    )
    .fetch_optional(state.app_state.db.pool())
    .await
    .map_err(|e| internal_error(e))?;

    let config = match config {
        Some(c) => LlmConfig {
            id: c.0,
            provider: c.1,
            display_name: c.2,
            api_base_url: c.3,
            api_key: c.4,
            model: c.5,
            enabled: c.6,
            created_at: c.7,
            updated_at: c.8,
        },
        None => return Err(bad_request("未配置 LLM，请先在设置中配置")),
    };

    let registry = Arc::new(FunctionRegistry::new(state.app_state.clone()));
    let client = LlmClient::new(config, registry);

    // Build messages with optional context
    let mut messages = vec![
        ChatMessage {
            role: Role::System,
            content: Some(format!(
                "你是 FluxDNS 的 AI 助手，帮助用户管理 DNS 服务。{}",
                req.context.as_deref().unwrap_or("")
            )),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        },
    ];

    match client.process_message(&mut messages, req.message).await {
        Ok(reply) => Ok(Json(ChatResponse {
            reply,
            functions_called: vec![], // TODO: Track called functions
        })),
        Err(e) => {
            tracing::error!("Chat processing error: {:?}", e);
            // Return specific error message to frontend
            Err(ApiError {
                code: "LLM_ERROR".to_string(),
                message: e.to_string(),
                details: None,
            })
        }
    }
}

/// Get conversation history
async fn get_conversations(
    State(state): State<LlmState>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let convos = sqlx::query_as::<_, (i64, String, String, Option<String>, chrono::NaiveDateTime)>(
        "SELECT id, session_id, role, content, created_at FROM llm_conversations ORDER BY created_at DESC LIMIT 100"
    )
    .fetch_all(state.app_state.db.pool())
    .await
    .map_err(|e| internal_error(e))?;

    let result: Vec<serde_json::Value> = convos.into_iter().map(|(id, session, role, content, created)| {
        serde_json::json!({
            "id": id,
            "session_id": session,
            "role": role,
            "content": content,
            "created_at": created.to_string()
        })
    }).collect();

    Ok(Json(result))
}

/// Clear conversation history
async fn clear_conversations(
    State(state): State<LlmState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("DELETE FROM llm_conversations")
        .execute(state.app_state.db.pool())
        .await
        .map_err(|e| internal_error(e))?;

    Ok(Json(serde_json::json!({"success": true, "message": "对话历史已清空"})))
}

/// Response struct with masked API key
#[derive(Debug, Serialize)]
struct LlmConfigResponse {
    id: i64,
    provider: String,
    display_name: String,
    api_base_url: String,
    api_key: String,
    api_key_masked: String,
    model: String,
    enabled: bool,
}

/// Mask API key for display
fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        "********".to_string()
    } else {
        format!("{}...{}", &key[..4], &key[key.len()-4..])
    }
}

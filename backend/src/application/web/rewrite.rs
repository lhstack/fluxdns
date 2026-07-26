//! Rewrite Rules API module
//!
//! Implements REST API endpoints for DNS rewrite rule management.
//!
//! # Requirements
//!
//! - 8.8: Provide rewrite rule management interface
//! - 8.9: Store rewrite rule configuration in database

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::application::web::ApiError;
use crate::business::rewrite_business::RewriteBusiness;
use crate::infrastructure::common::AppError;
use crate::infrastructure::repository::{CreateRewriteRule, RewriteRule, UpdateRewriteRule};

/// Application state for rewrite rules API
#[derive(Clone)]
pub struct RewriteState {
    pub business: Arc<RewriteBusiness>,
}

/// Valid match types
const VALID_MATCH_TYPES: &[&str] = &["exact", "wildcard", "regex"];

/// Valid action types
const VALID_ACTION_TYPES: &[&str] = &["map_ip", "map_domain", "block"];

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

/// Create rewrite rule request with validation
#[derive(Debug, Clone, Deserialize)]
pub struct CreateRewriteRuleRequest {
    pub pattern: String,
    pub match_type: String,
    pub action_type: String,
    pub action_value: Option<String>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub description: Option<String>,
}

fn default_enabled() -> bool {
    true
}

/// Update rewrite rule request
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateRewriteRuleRequest {
    pub pattern: Option<String>,
    pub match_type: Option<String>,
    pub action_type: Option<String>,
    pub action_value: Option<String>,
    pub priority: Option<i32>,
    pub enabled: Option<bool>,
    pub description: Option<String>,
}

/// API response wrapper for single rule
#[derive(Debug, Serialize)]
pub struct RewriteRuleResponse {
    pub data: RewriteRule,
}

/// API response wrapper for multiple rules
#[derive(Debug, Serialize)]
pub struct RewriteRulesListResponse {
    pub data: Vec<RewriteRule>,
    pub total: usize,
}

/// Validate pattern based on match type
fn validate_pattern(pattern: &str, match_type: &str) -> Result<(), String> {
    if pattern.is_empty() {
        return Err("Pattern cannot be empty".to_string());
    }
    if pattern.len() > 255 {
        return Err("Pattern cannot exceed 255 characters".to_string());
    }

    match match_type.to_lowercase().as_str() {
        "regex" => {
            // Validate regex pattern
            if regex::Regex::new(pattern).is_err() {
                return Err("Invalid regular expression pattern".to_string());
            }
        }
        "wildcard" => {
            // Wildcard patterns should contain at least one *
            if !pattern.contains('*') {
                return Err("Wildcard pattern must contain at least one '*'".to_string());
            }
        }
        "exact" => {
            // Exact patterns should be valid domain names
            let valid_chars = pattern
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_');
            if !valid_chars {
                return Err("Exact pattern contains invalid characters".to_string());
            }
        }
        _ => {}
    }
    Ok(())
}

/// Validate match type
fn validate_match_type(match_type: &str) -> Result<(), String> {
    let lower = match_type.to_lowercase();
    if !VALID_MATCH_TYPES.contains(&lower.as_str()) {
        return Err(format!(
            "Invalid match type. Must be one of: {}",
            VALID_MATCH_TYPES.join(", ")
        ));
    }
    Ok(())
}

/// Validate action type and value
fn validate_action(action_type: &str, action_value: &Option<String>) -> Result<(), String> {
    let lower = action_type.to_lowercase();
    if !VALID_ACTION_TYPES.contains(&lower.as_str()) {
        return Err(format!(
            "Invalid action type. Must be one of: {}",
            VALID_ACTION_TYPES.join(", ")
        ));
    }

    match lower.as_str() {
        "map_ip" => {
            let value = action_value
                .as_ref()
                .ok_or("action_value is required for map_ip action")?;
            if value.parse::<std::net::IpAddr>().is_err() {
                return Err("Invalid IP address for map_ip action".to_string());
            }
        }
        "map_domain" => {
            let value = action_value
                .as_ref()
                .ok_or("action_value is required for map_domain action")?;
            if value.is_empty() {
                return Err("action_value cannot be empty for map_domain action".to_string());
            }
        }
        "block" => {
            // Block action doesn't require a value
        }
        _ => {}
    }
    Ok(())
}

impl CreateRewriteRuleRequest {
    /// Validate the create request
    pub fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = Vec::new();

        if let Err(e) = validate_match_type(&self.match_type) {
            errors.push(ValidationError {
                field: "match_type".to_string(),
                message: e,
            });
        }

        // Only validate pattern if match_type is valid
        if validate_match_type(&self.match_type).is_ok() {
            if let Err(e) = validate_pattern(&self.pattern, &self.match_type) {
                errors.push(ValidationError {
                    field: "pattern".to_string(),
                    message: e,
                });
            }
        } else if self.pattern.is_empty() {
            errors.push(ValidationError {
                field: "pattern".to_string(),
                message: "Pattern cannot be empty".to_string(),
            });
        }

        if let Err(e) = validate_action(&self.action_type, &self.action_value) {
            errors.push(ValidationError {
                field: "action_type".to_string(),
                message: e,
            });
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationErrors { errors })
        }
    }

    /// Convert to CreateRewriteRule with normalized values
    pub fn into_create_rewrite_rule(self) -> CreateRewriteRule {
        CreateRewriteRule {
            pattern: self.pattern,
            match_type: self.match_type.to_lowercase(),
            action_type: self.action_type.to_lowercase(),
            action_value: self.action_value,
            priority: self.priority,
            enabled: self.enabled,
            description: self.description,
        }
    }
}

impl UpdateRewriteRuleRequest {
    /// Validate the update request
    pub fn validate(&self, existing: &RewriteRule) -> Result<(), ValidationErrors> {
        let mut errors = Vec::new();

        if let Some(ref match_type) = self.match_type {
            if let Err(e) = validate_match_type(match_type) {
                errors.push(ValidationError {
                    field: "match_type".to_string(),
                    message: e,
                });
            }
        }

        // Validate pattern against match type (new or existing)
        if let Some(ref pattern) = self.pattern {
            let match_type = self.match_type.as_deref().unwrap_or(&existing.match_type);
            if validate_match_type(match_type).is_ok() {
                if let Err(e) = validate_pattern(pattern, match_type) {
                    errors.push(ValidationError {
                        field: "pattern".to_string(),
                        message: e,
                    });
                }
            }
        }

        // Validate action if either action_type or action_value is provided
        if self.action_type.is_some() || self.action_value.is_some() {
            let action_type = self.action_type.as_deref().unwrap_or(&existing.action_type);
            let action_value = if self.action_value.is_some() {
                &self.action_value
            } else {
                &existing.action_value
            };
            if let Err(e) = validate_action(action_type, action_value) {
                errors.push(ValidationError {
                    field: "action_type".to_string(),
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

    /// Convert to UpdateRewriteRule with normalized values
    pub fn into_update_rewrite_rule(self) -> UpdateRewriteRule {
        UpdateRewriteRule {
            pattern: self.pattern,
            match_type: self.match_type.map(|t| t.to_lowercase()),
            action_type: self.action_type.map(|t| t.to_lowercase()),
            action_value: self.action_value,
            priority: self.priority,
            enabled: self.enabled,
            description: self.description,
        }
    }
}

/// List all rewrite rules
///
/// GET /api/rewrite
/// List all rewrite rules
///
/// GET /api/rewrite
pub async fn list_rules(State(state): State<RewriteState>) -> Result<impl IntoResponse, ApiError> {
    let rules = state.business.list().await?;
    Ok(Json(RewriteRulesListResponse {
        total: rules.len(),
        data: rules,
    }))
}

/// Get a rewrite rule by ID
///
/// GET /api/rewrite/:id
pub async fn get_rule(
    State(state): State<RewriteState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let rule = state.business.get(id).await?;
    Ok(Json(RewriteRuleResponse { data: rule }))
}

/// Create a new rewrite rule
///
/// POST /api/rewrite
pub async fn create_rule(
    State(state): State<RewriteState>,
    Json(request): Json<CreateRewriteRuleRequest>,
) -> Result<impl IntoResponse, ApiError> {
    request.validate().map_err(ApiError::validation)?;

    let rule = state
        .business
        .create(request.into_create_rewrite_rule())
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(RewriteRuleResponse { data: rule }),
    ))
}

/// Update a rewrite rule
///
/// PUT /api/rewrite/:id
pub async fn update_rule(
    State(state): State<RewriteState>,
    Path(id): Path<i64>,
    Json(request): Json<UpdateRewriteRuleRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let existing = state.business.get(id).await?;
    request.validate(&existing).map_err(ApiError::validation)?;

    let rule = state
        .business
        .update(id, request.into_update_rewrite_rule())
        .await?;

    Ok(Json(RewriteRuleResponse { data: rule }))
}

/// Delete a rewrite rule
///
/// DELETE /api/rewrite/:id
pub async fn delete_rule(
    State(state): State<RewriteState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    state.business.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Reload rewrite rules from database
///
/// POST /api/rewrite/reload
pub async fn reload_rules(
    State(state): State<RewriteState>,
) -> Result<impl IntoResponse, ApiError> {
    let count = state.business.reload().await?;

    Ok(Json(serde_json::json!({
        "message": "Rewrite rules reloaded successfully",
        "rules": count
    })))
}

/// Batch create rewrite rules request
#[derive(Debug, Clone, Deserialize)]
pub struct BatchCreateRequest {
    /// List of domain patterns (one per line or comma-separated)
    pub patterns: String,
    /// Match type for all rules
    #[serde(default = "default_match_type")]
    pub match_type: String,
    /// Action type for all rules
    #[serde(default = "default_action_type")]
    pub action_type: String,
    /// Action value (for map_ip or map_domain)
    pub action_value: Option<String>,
    /// Priority for all rules
    #[serde(default)]
    pub priority: i32,
    /// Whether rules are enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Description for all rules
    pub description: Option<String>,
}

fn default_match_type() -> String {
    "exact".to_string()
}

fn default_action_type() -> String {
    "block".to_string()
}

/// Batch create response
#[derive(Debug, Serialize)]
pub struct BatchCreateResponse {
    pub created: i64,
    pub message: String,
}

impl BatchCreateRequest {
    /// Split the raw pattern blob and validate every entry.
    fn into_rules(self) -> Result<Vec<CreateRewriteRule>, String> {
        validate_match_type(&self.match_type)?;
        validate_action(&self.action_type, &self.action_value)?;

        let patterns: Vec<String> = self
            .patterns
            .split(|c| c == '\n' || c == ',' || c == ';')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if patterns.is_empty() {
            return Err("No valid patterns provided".to_string());
        }

        for pattern in &patterns {
            validate_pattern(pattern, &self.match_type)
                .map_err(|e| format!("Invalid pattern '{}': {}", pattern, e))?;
        }

        let match_type = self.match_type.to_lowercase();
        let action_type = self.action_type.to_lowercase();

        Ok(patterns
            .into_iter()
            .map(|pattern| CreateRewriteRule {
                pattern,
                match_type: match_type.clone(),
                action_type: action_type.clone(),
                action_value: self.action_value.clone(),
                priority: self.priority,
                enabled: self.enabled,
                description: self.description.clone(),
            })
            .collect())
    }
}

/// Batch create rewrite rules
///
/// POST /api/rewrite/batch
///
/// Accepts a list of domain patterns (newline or comma separated) and creates
/// rewrite rules for each one with the same action.
pub async fn batch_create_rules(
    State(state): State<RewriteState>,
    Json(request): Json<BatchCreateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let rules = request
        .into_rules()
        .map_err(|e| ApiError::from(AppError::Validation(e)))?;

    let created = state.business.batch_create(rules).await?;

    Ok((
        StatusCode::CREATED,
        Json(BatchCreateResponse {
            created,
            message: format!("Successfully created {} rewrite rules", created),
        }),
    ))
}

/// Build the rewrite rules API router
pub fn rewrite_router(state: RewriteState) -> axum::Router {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/", get(list_rules).post(create_rule))
        .route("/batch", post(batch_create_rules))
        .route("/reload", post(reload_rules))
        .route("/:id", get(get_rule).put(update_rule).delete(delete_rule))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_match_type_valid() {
        assert!(validate_match_type("exact").is_ok());
        assert!(validate_match_type("EXACT").is_ok());
        assert!(validate_match_type("wildcard").is_ok());
        assert!(validate_match_type("regex").is_ok());
    }

    #[test]
    fn test_validate_match_type_invalid() {
        assert!(validate_match_type("invalid").is_err());
        assert!(validate_match_type("").is_err());
    }

    #[test]
    fn test_validate_pattern_exact() {
        assert!(validate_pattern("example.com", "exact").is_ok());
        assert!(validate_pattern("sub.example.com", "exact").is_ok());
        assert!(validate_pattern("", "exact").is_err());
    }

    #[test]
    fn test_validate_pattern_wildcard() {
        assert!(validate_pattern("*.example.com", "wildcard").is_ok());
        assert!(validate_pattern("*", "wildcard").is_ok());
        assert!(validate_pattern("example.com", "wildcard").is_err()); // No wildcard
    }

    #[test]
    fn test_validate_pattern_regex() {
        assert!(validate_pattern("^ads?\\.", "regex").is_ok());
        assert!(validate_pattern("[invalid", "regex").is_err()); // Invalid regex
    }

    #[test]
    fn test_validate_action_map_ip() {
        assert!(validate_action("map_ip", &Some("192.168.1.1".to_string())).is_ok());
        assert!(validate_action("map_ip", &Some("::1".to_string())).is_ok());
        assert!(validate_action("map_ip", &None).is_err());
        assert!(validate_action("map_ip", &Some("invalid".to_string())).is_err());
    }

    #[test]
    fn test_validate_action_map_domain() {
        assert!(validate_action("map_domain", &Some("example.com".to_string())).is_ok());
        assert!(validate_action("map_domain", &None).is_err());
        assert!(validate_action("map_domain", &Some("".to_string())).is_err());
    }

    #[test]
    fn test_validate_action_block() {
        assert!(validate_action("block", &None).is_ok());
        assert!(validate_action("block", &Some("ignored".to_string())).is_ok());
    }

    #[test]
    fn test_create_request_validation() {
        let valid_request = CreateRewriteRuleRequest {
            pattern: "*.ads.com".to_string(),
            match_type: "wildcard".to_string(),
            action_type: "block".to_string(),
            action_value: None,
            priority: 10,
            enabled: true,
            description: Some("Block ads".to_string()),
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = CreateRewriteRuleRequest {
            pattern: "".to_string(),
            match_type: "invalid".to_string(),
            action_type: "map_ip".to_string(),
            action_value: None,
            priority: 0,
            enabled: true,
            description: None,
        };
        let result = invalid_request.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_create_request_into_create_rewrite_rule() {
        let request = CreateRewriteRuleRequest {
            pattern: "*.example.com".to_string(),
            match_type: "WILDCARD".to_string(),
            action_type: "BLOCK".to_string(),
            action_value: None,
            priority: 10,
            enabled: true,
            description: None,
        };
        let create_rule = request.into_create_rewrite_rule();
        assert_eq!(create_rule.match_type, "wildcard");
        assert_eq!(create_rule.action_type, "block");
    }
}

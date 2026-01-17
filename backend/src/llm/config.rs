// LLM Configuration
// Database-backed configuration for LLM providers

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// LLM configuration stored in database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LlmConfig {
    pub id: i64,
    pub provider: String,
    pub display_name: String,
    pub api_base_url: String,
    pub api_key: String,
    pub model: String,
    pub enabled: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

/// Request to create/update LLM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfigRequest {
    pub provider: String,
    pub display_name: Option<String>,
    pub api_base_url: String,
    pub api_key: String,
    pub model: String,
}

/// Request to enable a specific LLM configuration
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnableConfigRequest {
    pub id: i64,
}

impl LlmConfig {
    /// Check if this configuration is valid for making API calls
    #[allow(dead_code)]
    pub fn is_valid(&self) -> bool {
        !self.api_base_url.is_empty() 
            && !self.api_key.is_empty() 
            && !self.model.is_empty()
    }
}

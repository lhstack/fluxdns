//! Query Strategy API module
//!
//! Implements REST API endpoints for DNS query strategy configuration.
//!
//! # Requirements
//!
//! - 3.11: Provide query strategy configuration interface
//! - 3.12: Store query strategy configuration in database

use std::sync::Arc;

use axum::{
    extract::State,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::dns::proxy::{ProxyManager, QueryStrategy};
use crate::web::ApiError;

/// Application state for strategy API
#[derive(Clone)]
pub struct StrategyState {
    pub db: Arc<Database>,
    pub proxy_manager: Arc<ProxyManager>,
}

/// Valid strategy types
const VALID_STRATEGIES: &[&str] = &["concurrent", "fastest", "round_robin", "random"];

/// Strategy configuration response
#[derive(Debug, Serialize)]
pub struct StrategyResponse {
    pub strategy: String,
    pub description: String,
}

impl From<QueryStrategy> for StrategyResponse {
    fn from(strategy: QueryStrategy) -> Self {
        let description = match strategy {
            QueryStrategy::Concurrent => "并发查询所有上游服务器，返回最快响应并取消其他请求",
            QueryStrategy::Fastest => "基于历史响应时间选择最快的服务器（首次查询自动使用并发策略探测）",
            QueryStrategy::RoundRobin => "按顺序轮流使用每个上游服务器",
            QueryStrategy::Random => "每次查询随机选择一个上游服务器",
        };
        Self {
            strategy: strategy.as_str().to_string(),
            description: description.to_string(),
        }
    }
}

/// Update strategy request
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateStrategyRequest {
    pub strategy: String,
}

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

impl UpdateStrategyRequest {
    /// Validate the update request
    pub fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = Vec::new();

        let lower = self.strategy.to_lowercase();
        if !VALID_STRATEGIES.contains(&lower.as_str()) && QueryStrategy::from_str(&self.strategy).is_none() {
            errors.push(ValidationError {
                field: "strategy".to_string(),
                message: format!(
                    "Invalid strategy. Must be one of: {}",
                    VALID_STRATEGIES.join(", ")
                ),
            });
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationErrors { errors })
        }
    }

    /// Get the strategy enum
    pub fn get_strategy(&self) -> Option<QueryStrategy> {
        QueryStrategy::from_str(&self.strategy)
    }
}

/// Available strategies response
#[derive(Debug, Serialize)]
pub struct AvailableStrategiesResponse {
    pub strategies: Vec<StrategyInfo>,
}

#[derive(Debug, Serialize)]
pub struct StrategyInfo {
    pub name: String,
    pub description: String,
}

/// Get current query strategy
///
/// GET /api/strategy
pub async fn get_strategy(
    State(state): State<StrategyState>,
) -> Result<impl IntoResponse, ApiError> {
    let strategy = state.proxy_manager.get_strategy().await;
    Ok(Json(StrategyResponse::from(strategy)))
}

/// Update query strategy
///
/// PUT /api/strategy
pub async fn update_strategy(
    State(state): State<StrategyState>,
    Json(request): Json<UpdateStrategyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate request
    if let Err(validation_errors) = request.validate() {
        return Err(ApiError {
            code: "BAD_REQUEST".to_string(),
            message: "Validation failed".to_string(),
            details: Some(serde_json::to_value(validation_errors).unwrap()),
        });
    }

    let strategy = request.get_strategy().ok_or_else(|| ApiError {
        code: "BAD_REQUEST".to_string(),
        message: "Invalid strategy".to_string(),
        details: None,
    })?;

    // Update strategy in proxy manager
    state.proxy_manager.set_strategy(strategy).await;

    // Persist to database
    let config_repo = state.db.system_config();
    if let Err(e) = config_repo.set("query_strategy", strategy.as_str()).await {
        tracing::warn!("Failed to persist query strategy: {}", e);
    }

    Ok(Json(StrategyResponse::from(strategy)))
}

/// Get available strategies
///
/// GET /api/strategy/available
pub async fn get_available_strategies() -> Result<impl IntoResponse, ApiError> {
    let strategies = vec![
        StrategyInfo {
            name: "concurrent".to_string(),
            description: "并发查询所有上游服务器，返回最快响应并取消其他请求".to_string(),
        },
        StrategyInfo {
            name: "fastest".to_string(),
            description: "基于历史响应时间选择最快的服务器（首次查询自动使用并发策略探测）".to_string(),
        },
        StrategyInfo {
            name: "round_robin".to_string(),
            description: "按顺序轮流使用每个上游服务器".to_string(),
        },
        StrategyInfo {
            name: "random".to_string(),
            description: "每次查询随机选择一个上游服务器".to_string(),
        },
    ];

    Ok(Json(AvailableStrategiesResponse { strategies }))
}

/// Build the strategy API router
pub fn strategy_router(state: StrategyState) -> axum::Router {
    use axum::routing::get;

    axum::Router::new()
        .route("/", get(get_strategy).put(update_strategy))
        .route("/available", get(get_available_strategies))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_response_from() {
        let response = StrategyResponse::from(QueryStrategy::Concurrent);
        assert_eq!(response.strategy, "concurrent");
        assert!(!response.description.is_empty());

        let response = StrategyResponse::from(QueryStrategy::Fastest);
        assert_eq!(response.strategy, "fastest");

        let response = StrategyResponse::from(QueryStrategy::RoundRobin);
        assert_eq!(response.strategy, "round_robin");

        let response = StrategyResponse::from(QueryStrategy::Random);
        assert_eq!(response.strategy, "random");
    }

    #[test]
    fn test_update_strategy_request_validation_valid() {
        let request = UpdateStrategyRequest {
            strategy: "concurrent".to_string(),
        };
        assert!(request.validate().is_ok());

        let request = UpdateStrategyRequest {
            strategy: "FASTEST".to_string(),
        };
        assert!(request.validate().is_ok());

        let request = UpdateStrategyRequest {
            strategy: "round_robin".to_string(),
        };
        assert!(request.validate().is_ok());

        let request = UpdateStrategyRequest {
            strategy: "random".to_string(),
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_update_strategy_request_validation_invalid() {
        let request = UpdateStrategyRequest {
            strategy: "invalid".to_string(),
        };
        assert!(request.validate().is_err());

        let request = UpdateStrategyRequest {
            strategy: "".to_string(),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_update_strategy_request_get_strategy() {
        let request = UpdateStrategyRequest {
            strategy: "concurrent".to_string(),
        };
        assert_eq!(request.get_strategy(), Some(QueryStrategy::Concurrent));

        let request = UpdateStrategyRequest {
            strategy: "fastest".to_string(),
        };
        assert_eq!(request.get_strategy(), Some(QueryStrategy::Fastest));

        let request = UpdateStrategyRequest {
            strategy: "invalid".to_string(),
        };
        assert_eq!(request.get_strategy(), None);
    }
}

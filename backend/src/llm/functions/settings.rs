// System Settings Functions

use async_trait::async_trait;
use serde_json::{json, Value};

use super::LlmFunction;
use crate::llm::types::{FunctionDefinition, FunctionResult};
use crate::state::AppState;

pub struct GetSystemStatusFunction;

#[async_trait]
impl LlmFunction for GetSystemStatusFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "get_system_status".to_string(),
            description: "获取系统运行状态".to_string(),
            parameters: json!({"type": "object", "properties": {}, "required": []}),
        }
    }

    async fn execute(&self, _args: Value, state: &AppState) -> FunctionResult {
        // Get query count
        let query_count = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM query_logs")
            .fetch_one(state.db.pool()).await.map(|r| r.0).unwrap_or(0);
        
        // Get upstream count
        let upstream_count = sqlx::query_as::<_, (i64, i64)>(
            "SELECT COUNT(*), SUM(CASE WHEN healthy = 1 THEN 1 ELSE 0 END) FROM upstream_servers"
        ).fetch_one(state.db.pool()).await.map(|r| (r.0, r.1)).unwrap_or((0, 0));

        FunctionResult::success(json!({
            "status": "running",
            "total_queries": query_count,
            "upstreams": {
                "total": upstream_count.0,
                "healthy": upstream_count.1
            }
        }))
    }
}

pub struct UpdateQueryStrategyFunction;

#[async_trait]
impl LlmFunction for UpdateQueryStrategyFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "update_query_strategy".to_string(),
            description: "更新 DNS 查询策略".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "strategy": {
                        "type": "string",
                        "enum": ["concurrent", "fastest", "round_robin", "random"]
                    }
                },
                "required": ["strategy"]
            }),
        }
    }

    async fn execute(&self, args: Value, _state: &AppState) -> FunctionResult {
        let strategy = args.get("strategy").and_then(|v| v.as_str()).unwrap_or("concurrent");
        // TODO: Apply strategy via ProxyManager
        FunctionResult::success(json!({"success": true, "strategy": strategy, "message": "策略更新功能待集成"}))
    }
}

pub struct ToggleRecordTypesFunction;

#[async_trait]
impl LlmFunction for ToggleRecordTypesFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "toggle_record_types".to_string(),
            description: "切换 DNS 记录类型的启用状态".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "types": {
                        "type": "object",
                        "description": "记录类型及其启用状态",
                        "additionalProperties": {"type": "boolean"}
                    }
                },
                "required": ["types"]
            }),
        }
    }

    async fn execute(&self, args: Value, _state: &AppState) -> FunctionResult {
        let types = args.get("types");
        FunctionResult::success(json!({"success": true, "types": types, "message": "记录类型切换功能待实现"}))
    }
}

pub struct ClearCacheFunction;

#[async_trait]
impl LlmFunction for ClearCacheFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "clear_cache".to_string(),
            description: "清空 DNS 缓存".to_string(),
            parameters: json!({"type": "object", "properties": {}, "required": []}),
        }
    }

    async fn execute(&self, _args: Value, _state: &AppState) -> FunctionResult {
        // TODO: Clear cache via cache module
        FunctionResult::success(json!({"success": true, "message": "缓存清空功能待集成"}))
    }
}

pub struct GetLogRetentionSettingsFunction;

#[async_trait]
impl LlmFunction for GetLogRetentionSettingsFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "get_log_retention_settings".to_string(),
            description: "获取日志保留设置".to_string(),
            parameters: json!({"type": "object", "properties": {}, "required": []}),
        }
    }

    async fn execute(&self, _args: Value, state: &AppState) -> FunctionResult {
        match sqlx::query_as::<_, (bool, i32)>(
            "SELECT auto_cleanup_enabled, retention_days FROM system_settings LIMIT 1"
        ).fetch_optional(state.db.pool()).await {
            Ok(Some((enabled, days))) => FunctionResult::success(json!({
                "auto_cleanup_enabled": enabled,
                "retention_days": days
            })),
            Ok(None) => FunctionResult::success(json!({
                "auto_cleanup_enabled": false,
                "retention_days": 30
            })),
            Err(e) => FunctionResult::error(format!("查询失败: {}", e)),
        }
    }
}

pub struct UpdateLogRetentionSettingsFunction;

#[async_trait]
impl LlmFunction for UpdateLogRetentionSettingsFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "update_log_retention_settings".to_string(),
            description: "更新日志保留设置".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "auto_cleanup_enabled": {"type": "boolean"},
                    "retention_days": {"type": "integer"}
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, args: Value, _state: &AppState) -> FunctionResult {
        let enabled = args.get("auto_cleanup_enabled").and_then(|v| v.as_bool());
        let days = args.get("retention_days").and_then(|v| v.as_i64());
        FunctionResult::success(json!({
            "success": true,
            "auto_cleanup_enabled": enabled,
            "retention_days": days,
            "message": "日志保留设置更新功能待实现"
        }))
    }
}

pub struct CleanupLogsBeforeDateFunction;

#[async_trait]
impl LlmFunction for CleanupLogsBeforeDateFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "cleanup_logs_before_date".to_string(),
            description: "清理指定日期之前的日志".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "before_date": {"type": "string", "description": "日期，格式 YYYY-MM-DD"}
                },
                "required": ["before_date"]
            }),
        }
    }

    async fn execute(&self, args: Value, state: &AppState) -> FunctionResult {
        let date = match args.get("before_date").and_then(|v| v.as_str()) {
            Some(d) => d,
            None => return FunctionResult::error("Missing required parameter: before_date"),
        };

        match sqlx::query("DELETE FROM query_logs WHERE DATE(created_at) < ?")
            .bind(date)
            .execute(state.db.pool())
            .await
        {
            Ok(result) => FunctionResult::success(json!({
                "success": true,
                "deleted_count": result.rows_affected()
            })),
            Err(e) => FunctionResult::error(format!("清理失败: {}", e)),
        }
    }
}

pub struct CleanupAllLogsFunction;

#[async_trait]
impl LlmFunction for CleanupAllLogsFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "cleanup_all_logs".to_string(),
            description: "清空所有查询日志".to_string(),
            parameters: json!({"type": "object", "properties": {}, "required": []}),
        }
    }

    async fn execute(&self, _args: Value, state: &AppState) -> FunctionResult {
        match sqlx::query("DELETE FROM query_logs").execute(state.db.pool()).await {
            Ok(result) => FunctionResult::success(json!({
                "success": true,
                "deleted_count": result.rows_affected()
            })),
            Err(e) => FunctionResult::error(format!("清空失败: {}", e)),
        }
    }
}

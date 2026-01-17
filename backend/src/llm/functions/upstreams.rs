// Upstream Servers Functions - Manage DNS upstream servers

use async_trait::async_trait;
use serde_json::{json, Value};

use super::LlmFunction;
use crate::llm::types::{FunctionDefinition, FunctionResult};
use crate::state::AppState;

/// Batch import upstream servers
pub struct BatchImportUpstreamsFunction;

#[async_trait]
impl LlmFunction for BatchImportUpstreamsFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "batch_import_upstreams".to_string(),
            description: "批量导入上游 DNS 服务器，支持多种协议（UDP/TCP/DoT/DoH/DoQ/DoH3）".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "servers": {
                        "type": "array",
                        "description": "要导入的上游服务器列表",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string", "description": "服务器名称"},
                                "address": {"type": "string", "description": "服务器地址"},
                                "protocol": {"type": "string", "enum": ["udp", "tcp", "dot", "doh", "doq", "doh3"]},
                                "timeout": {"type": "integer", "description": "超时时间（毫秒），默认 5000"}
                            },
                            "required": ["name", "address", "protocol"]
                        }
                    }
                },
                "required": ["servers"]
            }),
        }
    }

    async fn execute(&self, args: Value, state: &AppState) -> FunctionResult {
        let servers = match args.get("servers").and_then(|v| v.as_array()) {
            Some(s) => s,
            None => return FunctionResult::error("Missing required parameter: servers"),
        };

        let mut added = Vec::new();
        let mut errors = Vec::new();

        for server in servers {
            let name = server.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let address = server.get("address").and_then(|v| v.as_str()).unwrap_or("");
            let protocol = server.get("protocol").and_then(|v| v.as_str()).unwrap_or("udp");
            let timeout = server.get("timeout").and_then(|v| v.as_i64()).unwrap_or(5000) as i32;

            if name.is_empty() || address.is_empty() {
                errors.push(json!({"name": name, "error": "name 和 address 不能为空"}));
                continue;
            }

            let result = sqlx::query(
                "INSERT INTO upstream_servers (name, address, protocol, timeout, enabled) VALUES (?, ?, ?, ?, 1)"
            )
            .bind(name)
            .bind(address)
            .bind(protocol)
            .bind(timeout)
            .execute(state.db.pool())
            .await;

            match result {
                Ok(_) => added.push(json!({"name": name, "address": address, "protocol": protocol})),
                Err(e) => errors.push(json!({"name": name, "error": e.to_string()})),
            }
        }

        FunctionResult::success(json!({
            "added_count": added.len(),
            "error_count": errors.len(),
            "added": added,
            "errors": errors
        }))
    }
}

/// Edit an upstream server
pub struct EditUpstreamFunction;

#[async_trait]
impl LlmFunction for EditUpstreamFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "edit_upstream".to_string(),
            description: "编辑上游服务器配置".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": {"type": "integer", "description": "服务器 ID"},
                    "updates": {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"},
                            "address": {"type": "string"},
                            "protocol": {"type": "string"},
                            "timeout": {"type": "integer"},
                            "enabled": {"type": "boolean"}
                        }
                    }
                },
                "required": ["id", "updates"]
            }),
        }
    }

    async fn execute(&self, args: Value, state: &AppState) -> FunctionResult {
        let id = match args.get("id").and_then(|v| v.as_i64()) {
            Some(id) => id,
            None => return FunctionResult::error("Missing required parameter: id"),
        };

        let updates = match args.get("updates") {
            Some(u) => u,
            None => return FunctionResult::error("Missing required parameter: updates"),
        };

        let mut set_clauses = Vec::new();
        if let Some(name) = updates.get("name").and_then(|v| v.as_str()) {
            set_clauses.push(format!("name = '{}'", name));
        }
        if let Some(address) = updates.get("address").and_then(|v| v.as_str()) {
            set_clauses.push(format!("address = '{}'", address));
        }
        if let Some(protocol) = updates.get("protocol").and_then(|v| v.as_str()) {
            set_clauses.push(format!("protocol = '{}'", protocol));
        }
        if let Some(timeout) = updates.get("timeout").and_then(|v| v.as_i64()) {
            set_clauses.push(format!("timeout = {}", timeout));
        }
        if let Some(enabled) = updates.get("enabled").and_then(|v| v.as_bool()) {
            set_clauses.push(format!("enabled = {}", if enabled { 1 } else { 0 }));
        }

        if set_clauses.is_empty() {
            return FunctionResult::error("No valid updates provided");
        }

        let query = format!("UPDATE upstream_servers SET {} WHERE id = {}", set_clauses.join(", "), id);
        match sqlx::query(&query).execute(state.db.pool()).await {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    FunctionResult::success(json!({"success": true, "id": id}))
                } else {
                    FunctionResult::error(format!("未找到 ID 为 {} 的服务器", id))
                }
            }
            Err(e) => FunctionResult::error(format!("更新失败: {}", e)),
        }
    }
}

/// Delete an upstream server
pub struct DeleteUpstreamFunction;

#[async_trait]
impl LlmFunction for DeleteUpstreamFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "delete_upstream".to_string(),
            description: "删除上游服务器".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": {"type": "integer", "description": "服务器 ID"}
                },
                "required": ["id"]
            }),
        }
    }

    async fn execute(&self, args: Value, state: &AppState) -> FunctionResult {
        let id = match args.get("id").and_then(|v| v.as_i64()) {
            Some(id) => id,
            None => return FunctionResult::error("Missing required parameter: id"),
        };

        match sqlx::query("DELETE FROM upstream_servers WHERE id = ?").bind(id).execute(state.db.pool()).await {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    FunctionResult::success(json!({"success": true, "id": id, "message": "服务器已删除"}))
                } else {
                    FunctionResult::error(format!("未找到 ID 为 {} 的服务器", id))
                }
            }
            Err(e) => FunctionResult::error(format!("删除失败: {}", e)),
        }
    }
}

/// Check upstream server health
pub struct CheckUpstreamHealthFunction;

#[async_trait]
impl LlmFunction for CheckUpstreamHealthFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "check_upstream_health".to_string(),
            description: "检测单个上游服务器的健康状态".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": {"type": "integer", "description": "服务器 ID"}
                },
                "required": ["id"]
            }),
        }
    }

    async fn execute(&self, _args: Value, _state: &AppState) -> FunctionResult {
        // TODO: Implement actual health check using ProxyManager
        FunctionResult::success(json!({
            "message": "健康检查功能待实现，需要集成 ProxyManager"
        }))
    }
}

/// Batch check all upstream servers
pub struct BatchCheckUpstreamsFunction;

#[async_trait]
impl LlmFunction for BatchCheckUpstreamsFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "batch_check_upstreams".to_string(),
            description: "批量检测所有上游服务器的健康状态".to_string(),
            parameters: json!({"type": "object", "properties": {}, "required": []}),
        }
    }

    async fn execute(&self, _args: Value, _state: &AppState) -> FunctionResult {
        // TODO: Implement batch health check
        FunctionResult::success(json!({
            "message": "批量健康检查功能待实现"
        }))
    }
}

/// Reset upstream server health status
pub struct ResetUpstreamHealthFunction;

#[async_trait]
impl LlmFunction for ResetUpstreamHealthFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "reset_upstream_health".to_string(),
            description: "恢复不健康的上游服务器，将其标记为健康".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": {"type": "integer", "description": "服务器 ID"}
                },
                "required": ["id"]
            }),
        }
    }

    async fn execute(&self, args: Value, state: &AppState) -> FunctionResult {
        let id = match args.get("id").and_then(|v| v.as_i64()) {
            Some(id) => id,
            None => return FunctionResult::error("Missing required parameter: id"),
        };

        match sqlx::query("UPDATE upstream_servers SET healthy = 1 WHERE id = ?")
            .bind(id)
            .execute(state.db.pool())
            .await
        {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    FunctionResult::success(json!({"success": true, "id": id, "message": "服务器健康状态已恢复"}))
                } else {
                    FunctionResult::error(format!("未找到 ID 为 {} 的服务器", id))
                }
            }
            Err(e) => FunctionResult::error(format!("操作失败: {}", e)),
        }
    }
}

/// Analyze upstream servers performance
pub struct AnalyzeUpstreamsFunction;

#[async_trait]
impl LlmFunction for AnalyzeUpstreamsFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "analyze_upstreams".to_string(),
            description: "分析上游服务器的性能统计".to_string(),
            parameters: json!({"type": "object", "properties": {}, "required": []}),
        }
    }

    async fn execute(&self, _args: Value, state: &AppState) -> FunctionResult {
        match sqlx::query_as::<_, (i64, String, String, String, bool, i64, i64, f64)>(
            r#"
            SELECT id, name, address, protocol, healthy, 
                   COALESCE(query_count, 0) as query_count,
                   COALESCE(success_count, 0) as success_count,
                   COALESCE(avg_response_time, 0) as avg_response_time
            FROM upstream_servers
            LEFT JOIN upstream_stats ON upstream_servers.id = upstream_stats.server_id
            ORDER BY query_count DESC
            "#
        )
        .fetch_all(state.db.pool())
        .await
        {
            Ok(servers) => {
                let analysis: Vec<Value> = servers.iter().map(|s| {
                    let success_rate = if s.5 > 0 { (s.6 as f64 / s.5 as f64) * 100.0 } else { 0.0 };
                    json!({
                        "id": s.0,
                        "name": s.1,
                        "address": s.2,
                        "protocol": s.3,
                        "healthy": s.4,
                        "query_count": s.5,
                        "success_rate": format!("{:.1}%", success_rate),
                        "avg_response_time_ms": s.7
                    })
                }).collect();

                FunctionResult::success(json!({
                    "server_count": analysis.len(),
                    "servers": analysis
                }))
            }
            Err(e) => FunctionResult::error(format!("分析失败: {}", e)),
        }
    }
}

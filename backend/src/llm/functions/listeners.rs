// Listener Management Functions

use async_trait::async_trait;
use serde_json::{json, Value};

use super::LlmFunction;
use crate::llm::types::{FunctionDefinition, FunctionResult};
use crate::state::AppState;

pub struct ListListenersFunction;

#[async_trait]
impl LlmFunction for ListListenersFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "list_listeners".to_string(),
            description: "列出所有监听器配置".to_string(),
            parameters: json!({"type": "object", "properties": {}, "required": []}),
        }
    }

    async fn execute(&self, _args: Value, state: &AppState) -> FunctionResult {
        match sqlx::query_as::<_, (i64, String, String, i32, bool)>(
            "SELECT id, protocol, address, port, enabled FROM listeners"
        ).fetch_all(state.db.pool()).await {
            Ok(listeners) => {
                let list: Vec<Value> = listeners.iter().map(|l| json!({
                    "id": l.0, "protocol": l.1, "address": l.2, "port": l.3, "enabled": l.4
                })).collect();
                FunctionResult::success(json!({"count": list.len(), "listeners": list}))
            }
            Err(e) => FunctionResult::error(format!("查询失败: {}", e)),
        }
    }
}

pub struct AddListenerFunction;

#[async_trait]
impl LlmFunction for AddListenerFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "add_listener".to_string(),
            description: "添加新的监听器".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "protocol": {"type": "string", "enum": ["udp", "tcp", "doh", "dot"]},
                    "address": {"type": "string"},
                    "port": {"type": "integer"},
                    "enabled": {"type": "boolean"}
                },
                "required": ["protocol", "address", "port"]
            }),
        }
    }

    async fn execute(&self, args: Value, state: &AppState) -> FunctionResult {
        let protocol = args.get("protocol").and_then(|v| v.as_str()).unwrap_or("udp");
        let address = args.get("address").and_then(|v| v.as_str()).unwrap_or("0.0.0.0");
        let port = args.get("port").and_then(|v| v.as_i64()).unwrap_or(53) as i32;
        let enabled = args.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);

        match sqlx::query("INSERT INTO listeners (protocol, address, port, enabled) VALUES (?, ?, ?, ?)")
            .bind(protocol).bind(address).bind(port).bind(enabled)
            .execute(state.db.pool()).await
        {
            Ok(_) => FunctionResult::success(json!({"success": true, "message": "监听器已添加"})),
            Err(e) => FunctionResult::error(format!("添加失败: {}", e)),
        }
    }
}

pub struct EditListenerFunction;

#[async_trait]
impl LlmFunction for EditListenerFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "edit_listener".to_string(),
            description: "编辑监听器配置".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": {"type": "integer"},
                    "updates": {"type": "object"}
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
        let updates = args.get("updates");

        let mut set_clauses = Vec::new();
        if let Some(u) = updates {
            if let Some(v) = u.get("protocol").and_then(|v| v.as_str()) {
                set_clauses.push(format!("protocol = '{}'", v));
            }
            if let Some(v) = u.get("address").and_then(|v| v.as_str()) {
                set_clauses.push(format!("address = '{}'", v));
            }
            if let Some(v) = u.get("port").and_then(|v| v.as_i64()) {
                set_clauses.push(format!("port = {}", v));
            }
            if let Some(v) = u.get("enabled").and_then(|v| v.as_bool()) {
                set_clauses.push(format!("enabled = {}", if v { 1 } else { 0 }));
            }
        }

        if set_clauses.is_empty() {
            return FunctionResult::error("No valid updates");
        }

        let query = format!("UPDATE listeners SET {} WHERE id = {}", set_clauses.join(", "), id);
        match sqlx::query(&query).execute(state.db.pool()).await {
            Ok(r) => {
                if r.rows_affected() > 0 {
                    FunctionResult::success(json!({"success": true, "id": id}))
                } else {
                    FunctionResult::error("未找到该监听器")
                }
            }
            Err(e) => FunctionResult::error(format!("更新失败: {}", e)),
        }
    }
}

pub struct DeleteListenerFunction;

#[async_trait]
impl LlmFunction for DeleteListenerFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "delete_listener".to_string(),
            description: "删除监听器".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {"id": {"type": "integer"}},
                "required": ["id"]
            }),
        }
    }

    async fn execute(&self, args: Value, state: &AppState) -> FunctionResult {
        let id = match args.get("id").and_then(|v| v.as_i64()) {
            Some(id) => id,
            None => return FunctionResult::error("Missing required parameter: id"),
        };

        match sqlx::query("DELETE FROM listeners WHERE id = ?").bind(id).execute(state.db.pool()).await {
            Ok(r) => {
                if r.rows_affected() > 0 {
                    FunctionResult::success(json!({"success": true, "id": id}))
                } else {
                    FunctionResult::error("未找到该监听器")
                }
            }
            Err(e) => FunctionResult::error(format!("删除失败: {}", e)),
        }
    }
}

// DNS Records Functions - Manage local DNS records

use async_trait::async_trait;
use serde_json::{json, Value};

use super::LlmFunction;
use crate::llm::types::{FunctionDefinition, FunctionResult};
use crate::state::AppState;

/// Batch add DNS records
pub struct BatchAddDnsRecordsFunction;

#[async_trait]
impl LlmFunction for BatchAddDnsRecordsFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "batch_add_dns_records".to_string(),
            description: "批量添加 DNS 解析记录，支持多种记录类型（A/AAAA/CNAME/MX/TXT 等）".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "records": {
                        "type": "array",
                        "description": "要添加的 DNS 记录列表",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": {
                                    "type": "string",
                                    "description": "域名，如 example.com 或 *.example.com"
                                },
                                "record_type": {
                                    "type": "string",
                                    "description": "记录类型",
                                    "enum": ["A", "AAAA", "CNAME", "MX", "TXT", "PTR", "NS", "SRV"]
                                },
                                "value": {
                                    "type": "string",
                                    "description": "记录值，如 IP 地址或目标域名"
                                },
                                "ttl": {
                                    "type": "integer",
                                    "description": "TTL（生存时间），单位秒，默认 300"
                                },
                                "priority": {
                                    "type": "integer",
                                    "description": "优先级（仅 MX/SRV 记录需要）"
                                },
                                "enabled": {
                                    "type": "boolean",
                                    "description": "是否启用，默认 true"
                                }
                            },
                            "required": ["name", "record_type", "value"]
                        }
                    }
                },
                "required": ["records"]
            }),
        }
    }

    async fn execute(&self, args: Value, state: &AppState) -> FunctionResult {
        let records = match args.get("records").and_then(|v| v.as_array()) {
            Some(r) => r,
            None => return FunctionResult::error("Missing required parameter: records"),
        };

        let mut added = Vec::new();
        let mut errors = Vec::new();

        for record in records {
            let name = record.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let record_type = record.get("record_type").and_then(|v| v.as_str()).unwrap_or("A");
            let value = record.get("value").and_then(|v| v.as_str()).unwrap_or("");
            let ttl = record.get("ttl").and_then(|v| v.as_i64()).unwrap_or(300) as i32;
            let priority = record.get("priority").and_then(|v| v.as_i64()).map(|p| p as i32);
            let enabled = record.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);

            if name.is_empty() || value.is_empty() {
                errors.push(json!({"name": name, "error": "name 和 value 不能为空"}));
                continue;
            }

            let result = sqlx::query(
                r#"
                INSERT INTO dns_records (name, record_type, value, ttl, priority, enabled)
                VALUES (?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(name)
            .bind(record_type)
            .bind(value)
            .bind(ttl)
            .bind(priority)
            .bind(enabled)
            .execute(state.db.pool())
            .await;

            match result {
                Ok(_) => added.push(json!({"name": name, "type": record_type, "value": value})),
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

/// Edit a DNS record
pub struct EditDnsRecordFunction;

#[async_trait]
impl LlmFunction for EditDnsRecordFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "edit_dns_record".to_string(),
            description: "编辑现有的 DNS 记录".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "integer",
                        "description": "要编辑的记录 ID"
                    },
                    "updates": {
                        "type": "object",
                        "description": "要更新的字段",
                        "properties": {
                            "name": {"type": "string", "description": "新域名"},
                            "record_type": {"type": "string", "description": "新记录类型"},
                            "value": {"type": "string", "description": "新记录值"},
                            "ttl": {"type": "integer", "description": "新 TTL"},
                            "priority": {"type": "integer", "description": "新优先级"},
                            "enabled": {"type": "boolean", "description": "是否启用"}
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

        // Build dynamic update query
        let mut set_clauses = Vec::new();
        
        if let Some(name) = updates.get("name").and_then(|v| v.as_str()) {
            set_clauses.push(format!("name = '{}'", name));
        }
        if let Some(record_type) = updates.get("record_type").and_then(|v| v.as_str()) {
            set_clauses.push(format!("record_type = '{}'", record_type));
        }
        if let Some(value) = updates.get("value").and_then(|v| v.as_str()) {
            set_clauses.push(format!("value = '{}'", value));
        }
        if let Some(ttl) = updates.get("ttl").and_then(|v| v.as_i64()) {
            set_clauses.push(format!("ttl = {}", ttl));
        }
        if let Some(priority) = updates.get("priority").and_then(|v| v.as_i64()) {
            set_clauses.push(format!("priority = {}", priority));
        }
        if let Some(enabled) = updates.get("enabled").and_then(|v| v.as_bool()) {
            set_clauses.push(format!("enabled = {}", if enabled { 1 } else { 0 }));
        }

        if set_clauses.is_empty() {
            return FunctionResult::error("No valid updates provided");
        }

        let query = format!(
            "UPDATE dns_records SET {} WHERE id = {}",
            set_clauses.join(", "),
            id
        );

        match sqlx::query(&query).execute(state.db.pool()).await {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    FunctionResult::success(json!({"success": true, "id": id, "message": "记录已更新"}))
                } else {
                    FunctionResult::error(format!("未找到 ID 为 {} 的记录", id))
                }
            }
            Err(e) => FunctionResult::error(format!("更新失败: {}", e)),
        }
    }
}

/// Delete a DNS record
pub struct DeleteDnsRecordFunction;

#[async_trait]
impl LlmFunction for DeleteDnsRecordFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "delete_dns_record".to_string(),
            description: "删除 DNS 记录".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "integer",
                        "description": "要删除的记录 ID"
                    }
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

        match sqlx::query("DELETE FROM dns_records WHERE id = ?")
            .bind(id)
            .execute(state.db.pool())
            .await
        {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    FunctionResult::success(json!({"success": true, "id": id, "message": "记录已删除"}))
                } else {
                    FunctionResult::error(format!("未找到 ID 为 {} 的记录", id))
                }
            }
            Err(e) => FunctionResult::error(format!("删除失败: {}", e)),
        }
    }
}

/// List DNS records
pub struct ListDnsRecordsFunction;

#[async_trait]
impl LlmFunction for ListDnsRecordsFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "list_dns_records".to_string(),
            description: "列出 DNS 记录，支持按域名和类型筛选".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "按域名筛选（支持模糊匹配）"
                    },
                    "record_type": {
                        "type": "string",
                        "description": "按记录类型筛选"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "返回数量限制，默认 50"
                    }
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, args: Value, state: &AppState) -> FunctionResult {
        let name_filter = args.get("name").and_then(|v| v.as_str());
        let type_filter = args.get("record_type").and_then(|v| v.as_str());
        let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(50);

        let mut query = "SELECT id, name, record_type, value, ttl, priority, enabled FROM dns_records WHERE 1=1".to_string();
        
        if let Some(name) = name_filter {
            query.push_str(&format!(" AND name LIKE '%{}%'", name));
        }
        if let Some(record_type) = type_filter {
            query.push_str(&format!(" AND record_type = '{}'", record_type));
        }
        query.push_str(&format!(" LIMIT {}", limit));

        match sqlx::query_as::<_, (i64, String, String, String, i32, Option<i32>, bool)>(&query)
            .fetch_all(state.db.pool())
            .await
        {
            Ok(records) => {
                let records_json: Vec<Value> = records.iter().map(|r| json!({
                    "id": r.0,
                    "name": r.1,
                    "type": r.2,
                    "value": r.3,
                    "ttl": r.4,
                    "priority": r.5,
                    "enabled": r.6
                })).collect();

                FunctionResult::success(json!({
                    "count": records_json.len(),
                    "records": records_json
                }))
            }
            Err(e) => FunctionResult::error(format!("查询失败: {}", e)),
        }
    }
}

// Rewrite Rules Functions - Manage DNS query rewrite rules

use async_trait::async_trait;
use serde_json::{json, Value};

use super::LlmFunction;
use crate::llm::types::{FunctionDefinition, FunctionResult};
use crate::state::AppState;

/// Batch add rewrite rules
pub struct BatchAddRewriteRulesFunction;

#[async_trait]
impl LlmFunction for BatchAddRewriteRulesFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "batch_add_rewrite_rules".to_string(),
            description: "批量添加重写规则，支持精确匹配、通配符和正则表达式".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "rules": {
                        "type": "array",
                        "description": "要添加的重写规则列表",
                        "items": {
                            "type": "object",
                            "properties": {
                                "pattern": {
                                    "type": "string",
                                    "description": "匹配模式，如 ads.example.com 或 *.ad.* 或正则表达式"
                                },
                                "match_type": {
                                    "type": "string",
                                    "description": "匹配类型",
                                    "enum": ["exact", "wildcard", "regex"]
                                },
                                "action_type": {
                                    "type": "string",
                                    "description": "动作类型",
                                    "enum": ["block", "map_ip", "map_domain"]
                                },
                                "action_value": {
                                    "type": "string",
                                    "description": "动作值（仅 map_ip/map_domain 需要）"
                                },
                                "priority": {
                                    "type": "integer",
                                    "description": "优先级，数字越小优先级越高，默认 100"
                                },
                                "enabled": {
                                    "type": "boolean",
                                    "description": "是否启用，默认 true"
                                }
                            },
                            "required": ["pattern", "match_type", "action_type"]
                        }
                    }
                },
                "required": ["rules"]
            }),
        }
    }

    async fn execute(&self, args: Value, state: &AppState) -> FunctionResult {
        let rules = match args.get("rules").and_then(|v| v.as_array()) {
            Some(r) => r,
            None => return FunctionResult::error("Missing required parameter: rules"),
        };

        let mut added = Vec::new();
        let mut errors = Vec::new();

        for rule in rules {
            let pattern = rule.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
            let match_type = rule.get("match_type").and_then(|v| v.as_str()).unwrap_or("exact");
            let action_type = rule.get("action_type").and_then(|v| v.as_str()).unwrap_or("block");
            let action_value = rule.get("action_value").and_then(|v| v.as_str());
            let priority = rule.get("priority").and_then(|v| v.as_i64()).unwrap_or(100) as i32;
            let enabled = rule.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);

            if pattern.is_empty() {
                errors.push(json!({"pattern": pattern, "error": "pattern 不能为空"}));
                continue;
            }

            // Validate action_value for map types
            if (action_type == "map_ip" || action_type == "map_domain") && action_value.is_none() {
                errors.push(json!({"pattern": pattern, "error": "map_ip/map_domain 需要 action_value"}));
                continue;
            }

            let result = sqlx::query(
                r#"
                INSERT INTO rewrite_rules (pattern, match_type, action_type, action_value, priority, enabled)
                VALUES (?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(pattern)
            .bind(match_type)
            .bind(action_type)
            .bind(action_value)
            .bind(priority)
            .bind(enabled)
            .execute(state.db.pool())
            .await;

            match result {
                Ok(_) => added.push(json!({
                    "pattern": pattern,
                    "match_type": match_type,
                    "action_type": action_type
                })),
                Err(e) => errors.push(json!({"pattern": pattern, "error": e.to_string()})),
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

/// Edit a rewrite rule
pub struct EditRewriteRuleFunction;

#[async_trait]
impl LlmFunction for EditRewriteRuleFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "edit_rewrite_rule".to_string(),
            description: "编辑现有的重写规则".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "integer",
                        "description": "要编辑的规则 ID"
                    },
                    "updates": {
                        "type": "object",
                        "description": "要更新的字段",
                        "properties": {
                            "pattern": {"type": "string"},
                            "match_type": {"type": "string", "enum": ["exact", "wildcard", "regex"]},
                            "action_type": {"type": "string", "enum": ["block", "map_ip", "map_domain"]},
                            "action_value": {"type": "string"},
                            "priority": {"type": "integer"},
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
        
        if let Some(pattern) = updates.get("pattern").and_then(|v| v.as_str()) {
            set_clauses.push(format!("pattern = '{}'", pattern));
        }
        if let Some(match_type) = updates.get("match_type").and_then(|v| v.as_str()) {
            set_clauses.push(format!("match_type = '{}'", match_type));
        }
        if let Some(action_type) = updates.get("action_type").and_then(|v| v.as_str()) {
            set_clauses.push(format!("action_type = '{}'", action_type));
        }
        if let Some(action_value) = updates.get("action_value").and_then(|v| v.as_str()) {
            set_clauses.push(format!("action_value = '{}'", action_value));
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
            "UPDATE rewrite_rules SET {} WHERE id = {}",
            set_clauses.join(", "),
            id
        );

        match sqlx::query(&query).execute(state.db.pool()).await {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    FunctionResult::success(json!({"success": true, "id": id, "message": "规则已更新"}))
                } else {
                    FunctionResult::error(format!("未找到 ID 为 {} 的规则", id))
                }
            }
            Err(e) => FunctionResult::error(format!("更新失败: {}", e)),
        }
    }
}

/// Delete a rewrite rule
pub struct DeleteRewriteRuleFunction;

#[async_trait]
impl LlmFunction for DeleteRewriteRuleFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "delete_rewrite_rule".to_string(),
            description: "删除重写规则".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "integer",
                        "description": "要删除的规则 ID"
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

        match sqlx::query("DELETE FROM rewrite_rules WHERE id = ?")
            .bind(id)
            .execute(state.db.pool())
            .await
        {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    FunctionResult::success(json!({"success": true, "id": id, "message": "规则已删除"}))
                } else {
                    FunctionResult::error(format!("未找到 ID 为 {} 的规则", id))
                }
            }
            Err(e) => FunctionResult::error(format!("删除失败: {}", e)),
        }
    }
}

/// List rewrite rules
pub struct ListRewriteRulesFunction;

#[async_trait]
impl LlmFunction for ListRewriteRulesFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "list_rewrite_rules".to_string(),
            description: "列出重写规则，支持筛选".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "按匹配模式筛选"
                    },
                    "action_type": {
                        "type": "string",
                        "description": "按动作类型筛选",
                        "enum": ["block", "map_ip", "map_domain"]
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
        let pattern_filter = args.get("pattern").and_then(|v| v.as_str());
        let action_filter = args.get("action_type").and_then(|v| v.as_str());
        let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(50);

        let mut query = "SELECT id, pattern, match_type, action_type, action_value, priority, enabled FROM rewrite_rules WHERE 1=1".to_string();
        
        if let Some(pattern) = pattern_filter {
            query.push_str(&format!(" AND pattern LIKE '%{}%'", pattern));
        }
        if let Some(action_type) = action_filter {
            query.push_str(&format!(" AND action_type = '{}'", action_type));
        }
        query.push_str(" ORDER BY priority ASC");
        query.push_str(&format!(" LIMIT {}", limit));

        match sqlx::query_as::<_, (i64, String, String, String, Option<String>, i32, bool)>(&query)
            .fetch_all(state.db.pool())
            .await
        {
            Ok(rules) => {
                let rules_json: Vec<Value> = rules.iter().map(|r| json!({
                    "id": r.0,
                    "pattern": r.1,
                    "match_type": r.2,
                    "action_type": r.3,
                    "action_value": r.4,
                    "priority": r.5,
                    "enabled": r.6
                })).collect();

                FunctionResult::success(json!({
                    "count": rules_json.len(),
                    "rules": rules_json
                }))
            }
            Err(e) => FunctionResult::error(format!("查询失败: {}", e)),
        }
    }
}

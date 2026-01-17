// Analytics Functions - Advanced analysis and security detection

use async_trait::async_trait;
use serde_json::{json, Value};

use super::LlmFunction;
use crate::llm::types::{FunctionDefinition, FunctionResult};
use crate::state::AppState;

pub struct GetClientStatsFunction;

#[async_trait]
impl LlmFunction for GetClientStatsFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "get_client_stats".to_string(),
            description: "获取客户端查询统计（按 IP 分组）".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "time_range": {"type": "string"},
                    "limit": {"type": "integer"}
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, args: Value, state: &AppState) -> FunctionResult {
        let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(20);
        
        match sqlx::query_as::<_, (String, i64)>(
            "SELECT client_ip, COUNT(*) as count FROM query_logs GROUP BY client_ip ORDER BY count DESC LIMIT ?"
        ).bind(limit).fetch_all(state.db.pool()).await {
            Ok(clients) => {
                let stats: Vec<Value> = clients.iter().map(|(ip, count)| {
                    json!({"client_ip": ip, "query_count": count})
                }).collect();
                FunctionResult::success(json!({"clients": stats}))
            }
            Err(e) => FunctionResult::error(format!("查询失败: {}", e)),
        }
    }
}

pub struct DetectDnsTunnelingFunction;

#[async_trait]
impl LlmFunction for DetectDnsTunnelingFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "detect_dns_tunneling".to_string(),
            description: "检测 DNS 隧道攻击（异常长域名/高频 TXT 查询）".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "time_range": {"type": "string"}
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, _args: Value, state: &AppState) -> FunctionResult {
        // Check for long domain names (potential tunneling)
        let long_domains = sqlx::query_as::<_, (String, i64)>(
            "SELECT query_name, LENGTH(query_name) as len FROM query_logs WHERE LENGTH(query_name) > 50 GROUP BY query_name LIMIT 10"
        ).fetch_all(state.db.pool()).await.unwrap_or_default();

        // Check for high TXT query rate
        let txt_count = sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(*) FROM query_logs WHERE query_type = 'TXT'"
        ).fetch_one(state.db.pool()).await.map(|r| r.0).unwrap_or(0);

        let mut findings: Vec<Value> = Vec::new();
        
        for (domain, len) in long_domains {
            findings.push(json!({
                "type": "long_domain",
                "severity": "medium",
                "domain": domain,
                "length": len
            }));
        }

        if txt_count > 1000 {
            findings.push(json!({
                "type": "high_txt_queries",
                "severity": "high",
                "count": txt_count,
                "description": "TXT 查询数量异常高"
            }));
        }

        FunctionResult::success(json!({
            "finding_count": findings.len(),
            "findings": findings
        }))
    }
}

pub struct SuggestBlockingRulesFunction;

#[async_trait]
impl LlmFunction for SuggestBlockingRulesFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "suggest_blocking_rules".to_string(),
            description: "基于日志分析智能推荐阻止规则".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "limit": {"type": "integer"}
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, args: Value, state: &AppState) -> FunctionResult {
        let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(10);

        // Find domains with high NXDOMAIN rate
        let suspicious = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT query_name, COUNT(*) as count 
            FROM query_logs 
            WHERE response_code = 'NXDOMAIN'
            GROUP BY query_name 
            HAVING count > 5
            ORDER BY count DESC
            LIMIT ?
            "#
        ).bind(limit).fetch_all(state.db.pool()).await.unwrap_or_default();

        let suggestions: Vec<Value> = suspicious.iter().map(|(domain, count)| {
            json!({
                "domain": domain,
                "nxdomain_count": count,
                "suggested_rule": {
                    "pattern": domain,
                    "match_type": "exact",
                    "action_type": "block"
                }
            })
        }).collect();

        FunctionResult::success(json!({
            "suggestion_count": suggestions.len(),
            "suggestions": suggestions
        }))
    }
}

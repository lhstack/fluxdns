// Log Analysis Functions - Analyze DNS query logs

use async_trait::async_trait;
use serde_json::{json, Value};

use super::LlmFunction;
use crate::llm::types::{FunctionDefinition, FunctionResult};
use crate::state::AppState;

/// Analyze query logs
pub struct AnalyzeQueryLogsFunction;

#[async_trait]
impl LlmFunction for AnalyzeQueryLogsFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "analyze_query_logs".to_string(),
            description: "综合分析查询日志".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "time_range": {
                        "type": "string",
                        "description": "时间范围，如 '1h', '24h', '7d'"
                    }
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, args: Value, state: &AppState) -> FunctionResult {
        let time_range = args.get("time_range").and_then(|v| v.as_str()).unwrap_or("24h");
        let interval = parse_time_range(time_range);

        let stats = sqlx::query_as::<_, (i64, i64, i64, f64)>(
            r#"
            SELECT 
                COUNT(*) as total,
                SUM(CASE WHEN cache_hit = 1 THEN 1 ELSE 0 END) as cache_hits,
                SUM(CASE WHEN response_code = 'NOERROR' THEN 1 ELSE 0 END) as success,
                AVG(response_time) as avg_response_time
            FROM query_logs
            WHERE created_at > datetime('now', ?)
            "#
        )
        .bind(interval)
        .fetch_one(state.db.pool())
        .await;

        match stats {
            Ok((total, cache_hits, success, avg_time)) => {
                let cache_rate = if total > 0 { (cache_hits as f64 / total as f64) * 100.0 } else { 0.0 };
                let success_rate = if total > 0 { (success as f64 / total as f64) * 100.0 } else { 0.0 };

                FunctionResult::success(json!({
                    "time_range": time_range,
                    "total_queries": total,
                    "cache_hit_rate": format!("{:.1}%", cache_rate),
                    "success_rate": format!("{:.1}%", success_rate),
                    "avg_response_time_ms": format!("{:.2}", avg_time)
                }))
            }
            Err(e) => FunctionResult::error(format!("分析失败: {}", e)),
        }
    }
}

/// Get high frequency queries
pub struct GetHighFrequencyQueriesFunction;

#[async_trait]
impl LlmFunction for GetHighFrequencyQueriesFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "get_high_frequency_queries".to_string(),
            description: "获取高频查询排行".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "limit": {"type": "integer", "description": "返回数量，默认 10"},
                    "time_range": {"type": "string", "description": "时间范围"}
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, args: Value, state: &AppState) -> FunctionResult {
        let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(10);
        let time_range = args.get("time_range").and_then(|v| v.as_str()).unwrap_or("24h");
        let interval = parse_time_range(time_range);

        match sqlx::query_as::<_, (String, String, i64)>(
            r#"
            SELECT query_name, query_type, COUNT(*) as count
            FROM query_logs
            WHERE created_at > datetime('now', ?)
            GROUP BY query_name, query_type
            ORDER BY count DESC
            LIMIT ?
            "#
        )
        .bind(interval)
        .bind(limit)
        .fetch_all(state.db.pool())
        .await
        {
            Ok(results) => {
                let queries: Vec<Value> = results.iter().map(|(name, qtype, count)| {
                    json!({"domain": name, "type": qtype, "count": count})
                }).collect();

                FunctionResult::success(json!({
                    "time_range": time_range,
                    "top_queries": queries
                }))
            }
            Err(e) => FunctionResult::error(format!("查询失败: {}", e)),
        }
    }
}

/// Detect anomalies in query logs
pub struct DetectAnomaliesFunction;

#[async_trait]
impl LlmFunction for DetectAnomaliesFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "detect_anomalies".to_string(),
            description: "检测异常流量模式".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "time_range": {"type": "string", "description": "时间范围"}
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, args: Value, state: &AppState) -> FunctionResult {
        let time_range = args.get("time_range").and_then(|v| v.as_str()).unwrap_or("1h");
        let interval = parse_time_range(time_range);

        // Check for NXDOMAIN flood
        let nxdomain = sqlx::query_as::<_, (i64,)>(
            r#"
            SELECT COUNT(*) FROM query_logs 
            WHERE response_code = 'NXDOMAIN' AND created_at > datetime('now', ?)
            "#
        )
        .bind(&interval)
        .fetch_one(state.db.pool())
        .await
        .map(|r| r.0)
        .unwrap_or(0);

        // Check for high query rate from single client
        let client_flood = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT client_ip, COUNT(*) as count FROM query_logs
            WHERE created_at > datetime('now', ?)
            GROUP BY client_ip
            HAVING count > 1000
            "#
        )
        .bind(&interval)
        .fetch_all(state.db.pool())
        .await
        .unwrap_or_default();

        let mut anomalies = Vec::new();
        
        if nxdomain > 100 {
            anomalies.push(json!({
                "type": "nxdomain_flood",
                "severity": "high",
                "description": format!("检测到 {} 次 NXDOMAIN 响应，可能存在随机子域名攻击", nxdomain)
            }));
        }

        for (ip, count) in client_flood {
            anomalies.push(json!({
                "type": "client_flood",
                "severity": "medium",
                "description": format!("客户端 {} 产生了 {} 次查询", ip, count)
            }));
        }

        FunctionResult::success(json!({
            "time_range": time_range,
            "anomaly_count": anomalies.len(),
            "anomalies": anomalies
        }))
    }
}

/// Get domain query count
pub struct GetDomainQueryCountFunction;

#[async_trait]
impl LlmFunction for GetDomainQueryCountFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "get_domain_query_count".to_string(),
            description: "获取指定域名的查询数量".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "domain": {"type": "string", "description": "要查询的域名"},
                    "time_range": {"type": "string", "description": "时间范围"}
                },
                "required": ["domain"]
            }),
        }
    }

    async fn execute(&self, args: Value, state: &AppState) -> FunctionResult {
        let domain = match args.get("domain").and_then(|v| v.as_str()) {
            Some(d) => d,
            None => return FunctionResult::error("Missing required parameter: domain"),
        };
        let time_range = args.get("time_range").and_then(|v| v.as_str()).unwrap_or("24h");
        let interval = parse_time_range(time_range);

        match sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(*) FROM query_logs WHERE query_name LIKE ? AND created_at > datetime('now', ?)"
        )
        .bind(format!("%{}%", domain))
        .bind(interval)
        .fetch_one(state.db.pool())
        .await
        {
            Ok((count,)) => FunctionResult::success(json!({
                "domain": domain,
                "time_range": time_range,
                "query_count": count
            })),
            Err(e) => FunctionResult::error(format!("查询失败: {}", e)),
        }
    }
}

/// Get query ranking
pub struct GetQueryRankingFunction;

#[async_trait]
impl LlmFunction for GetQueryRankingFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "get_query_ranking".to_string(),
            description: "获取域名查询数量排名".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "limit": {"type": "integer", "description": "返回数量"},
                    "time_range": {"type": "string"},
                    "order": {"type": "string", "enum": ["desc", "asc"]}
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, args: Value, state: &AppState) -> FunctionResult {
        let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(20);
        let time_range = args.get("time_range").and_then(|v| v.as_str()).unwrap_or("24h");
        let order = args.get("order").and_then(|v| v.as_str()).unwrap_or("desc");
        let interval = parse_time_range(time_range);

        let order_clause = if order == "asc" { "ASC" } else { "DESC" };
        let query = format!(
            r#"
            SELECT query_name, COUNT(*) as count
            FROM query_logs
            WHERE created_at > datetime('now', ?)
            GROUP BY query_name
            ORDER BY count {}
            LIMIT ?
            "#,
            order_clause
        );

        match sqlx::query_as::<_, (String, i64)>(&query)
            .bind(interval)
            .bind(limit)
            .fetch_all(state.db.pool())
            .await
        {
            Ok(results) => {
                let ranking: Vec<Value> = results.iter().enumerate().map(|(i, (name, count))| {
                    json!({"rank": i + 1, "domain": name, "count": count})
                }).collect();

                FunctionResult::success(json!({
                    "time_range": time_range,
                    "ranking": ranking
                }))
            }
            Err(e) => FunctionResult::error(format!("查询失败: {}", e)),
        }
    }
}

/// Parse time range string to SQLite interval
fn parse_time_range(range: &str) -> String {
    match range {
        "1h" => "-1 hour".to_string(),
        "6h" => "-6 hours".to_string(),
        "12h" => "-12 hours".to_string(),
        "24h" | "1d" => "-1 day".to_string(),
        "7d" => "-7 days".to_string(),
        "30d" => "-30 days".to_string(),
        _ => "-1 day".to_string(),
    }
}

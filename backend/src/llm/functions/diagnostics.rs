// Diagnostics Functions

use async_trait::async_trait;
use serde_json::{json, Value};
use std::time::Instant;
use std::str::FromStr;

use super::LlmFunction;
use crate::llm::types::{FunctionDefinition, FunctionResult};
use crate::state::AppState;
use crate::dns::{DnsQuery, RecordType, CacheKey};
use crate::dns::proxy::{UpstreamServer, UpstreamProtocol};
use crate::dns::proxy::{
    UdpDnsClient, DotDnsClient, DohDnsClient, DoqDnsClient, Doh3DnsClient, DnsClient
};

pub struct TraceDnsResolutionFunction;

#[async_trait]
impl LlmFunction for TraceDnsResolutionFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "trace_dns_resolution".to_string(),
            description: "追踪 DNS 解析的完整过程，显示每一步的耗时和结果".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "domain": {"type": "string"},
                    "record_type": {"type": "string", "enum": ["A", "AAAA", "CNAME", "MX", "TXT", "NS", "PTR"]}
                },
                "required": ["domain", "record_type"]
            }),
        }
    }

    async fn execute(&self, args: Value, state: &AppState) -> FunctionResult {
        let domain = args.get("domain").and_then(|v| v.as_str()).unwrap_or("");
        let record_type_str = args.get("record_type").and_then(|v| v.as_str()).unwrap_or("A");
        
        let record_type: RecordType = match RecordType::from_str(record_type_str) {
            Ok(rt) => rt,
            Err(_) => return FunctionResult::error(format!("不支持的记录类型: {}", record_type_str)),
        };

        if domain.is_empty() {
            return FunctionResult::error("域名不能为空");
        }

        let mut steps = Vec::new();
        let query = DnsQuery::new(domain, record_type.clone());
        let total_start = Instant::now();

        // 1. 检查缓存
        let step_start = Instant::now();
        let cache_key = CacheKey::from_query(&query);
        let cache_result = if let Some(_) = state.cache.get(&cache_key).await {
            "Hit"
        } else {
            "Miss"
        };
        steps.push(json!({
            "step": 1,
            "component": "Cache",
            "action": "Check local cache",
            "status": cache_result,
            "latency_ms": step_start.elapsed().as_millis() as u64
        }));

        if cache_result == "Hit" {
            return FunctionResult::success(json!({
                "domain": domain,
                "type": record_type_str,
                "total_latency_ms": total_start.elapsed().as_millis() as u64,
                "result": "Resolved from Cache",
                "steps": steps
            }));
        }

        // 2. 检查重写规则
        let step_start = Instant::now();
        let rewrite_result = if let Some(rule) = state.rewrite_engine.check(domain).await {
            Some(format!("Matched Rule #{}", rule.rule_id))
        } else {
            None
        };
        steps.push(json!({
            "step": 2,
            "component": "RewriteEngine",
            "action": "Check rewrite rules",
            "status": rewrite_result.clone().unwrap_or_else(|| "No Match".to_string()),
            "latency_ms": step_start.elapsed().as_millis() as u64
        }));

        if let Some(rule_info) = rewrite_result {
             return FunctionResult::success(json!({
                "domain": domain,
                "type": record_type_str,
                "total_latency_ms": total_start.elapsed().as_millis() as u64,
                "result": rule_info,
                "steps": steps
            }));
        }

        // 3. 检查本地记录
        let step_start = Instant::now();
        let local_records = match state.db.dns_records().get_by_name_and_type_with_wildcard(domain, record_type_str).await {
            Ok(records) if !records.is_empty() => {
                let values: Vec<String> = records.iter().map(|r| r.value.clone()).collect();
                Some((format!("Found {} records", records.len()), values))
            },
            _ => None
        };
        steps.push(json!({
            "step": 3,
            "component": "LocalDB",
            "action": "Check local records",
            "status": local_records.as_ref().map(|r| r.0.clone()).unwrap_or_else(|| "No Records".to_string()),
            "latency_ms": step_start.elapsed().as_millis() as u64
        }));

        if let Some((record_info, values)) = local_records {
            return FunctionResult::success(json!({
                "domain": domain,
                "type": record_type_str,
                "total_latency_ms": total_start.elapsed().as_millis() as u64,
                "result": record_info,
                "answers": values,
                "steps": steps
            }));
        }

        // 4. 查询所有启用的上游服务器
        let step_start = Instant::now();
        let mut upstream_results = Vec::new();

        // 获取所有启用的上游服务器
        let db_servers = match state.db.upstream_servers().list_enabled().await {
            Ok(servers) => servers,
            Err(e) => {
                return FunctionResult::error(format!("Failed to get upstream servers: {}", e));
            }
        };

        if db_servers.is_empty() {
            return FunctionResult::error("没有启用的上游服务器");
        }

        for db_server in db_servers {
            let protocol = match UpstreamProtocol::from_str(&db_server.protocol) {
                Some(p) => p,
                None => continue,
            };

            let server_config = UpstreamServer::new(
                db_server.id,
                db_server.name.clone(),
                db_server.address.clone(),
                protocol,
                db_server.timeout as u32,
            );

            let client: Box<dyn DnsClient> = match protocol {
                UpstreamProtocol::Udp => Box::new(UdpDnsClient::new(server_config)),
                UpstreamProtocol::Dot => Box::new(DotDnsClient::new(server_config)),
                UpstreamProtocol::Doh => Box::new(DohDnsClient::new(server_config)),
                UpstreamProtocol::Doq => Box::new(DoqDnsClient::new(server_config)),
                UpstreamProtocol::Doh3 => Box::new(Doh3DnsClient::new(server_config)),
            };

            let query_start = Instant::now();
            let result = match client.query(&query).await {
                Ok(r) => {
                    let answers: Vec<String> = r.response.answers.iter().map(|a| a.value.clone()).collect();
                    json!({
                        "server": db_server.name,
                        "protocol": db_server.protocol,
                        "status": "Success",
                        "response_code": r.response.response_code.to_string(),
                        "latency_ms": query_start.elapsed().as_millis() as u64,
                        "answers": answers
                    })
                },
                Err(e) => json!({
                    "server": db_server.name,
                    "protocol": db_server.protocol,
                    "status": "Failed",
                    "error": e.to_string(),
                    "latency_ms": query_start.elapsed().as_millis() as u64
                })
            };
            upstream_results.push(result);
        }
        
        steps.push(json!({
            "step": 4,
            "component": "UpstreamServers",
            "action": "Query all enabled upstream servers",
            "server_count": upstream_results.len(),
            "latency_ms": step_start.elapsed().as_millis() as u64
        }));

        FunctionResult::success(json!({
            "domain": domain,
            "type": record_type_str,
            "total_latency_ms": total_start.elapsed().as_millis() as u64,
            "result": format!("Queried {} upstream servers", upstream_results.len()),
            "upstream_results": upstream_results,
            "steps": steps
        }))
    }
}

pub struct TestUpstreamConnectivityFunction;

#[async_trait]
impl LlmFunction for TestUpstreamConnectivityFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "test_upstream_connectivity".to_string(),
            description: "测试指定上游服务器的连通性 (Health Check)".to_string(),
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

        // 1. Get server config from DB
        let db_server = match state.db.upstream_servers().get_by_id(id).await {
            Ok(Some(s)) => s,
            Ok(None) => return FunctionResult::error("未找到指定的上游服务器"),
            Err(e) => return FunctionResult::error(format!("Database error: {}", e)),
        };

        let protocol = match UpstreamProtocol::from_str(&db_server.protocol) {
            Some(p) => p,
            None => return FunctionResult::error(format!("Invalid protocol: {}", db_server.protocol)),
        };

        let server_config = UpstreamServer::new(
            db_server.id,
            db_server.name.clone(),
            db_server.address.clone(),
            protocol,
            db_server.timeout as u32,
        );

        // 2. Instantiate Client
        let client: Box<dyn DnsClient> = match protocol {
            UpstreamProtocol::Udp => Box::new(UdpDnsClient::new(server_config)),
            UpstreamProtocol::Dot => Box::new(DotDnsClient::new(server_config)),
            UpstreamProtocol::Doh => Box::new(DohDnsClient::new(server_config)),
            UpstreamProtocol::Doq => Box::new(DoqDnsClient::new(server_config)),
            UpstreamProtocol::Doh3 => Box::new(Doh3DnsClient::new(server_config)),
        };

        // 3. Perform Health Check
        match client.health_check().await {
            Ok(duration) => {
                FunctionResult::success(json!({
                    "id": id,
                    "name": db_server.name,
                    "status": "Online",
                    "latency_ms": duration.as_millis() as u64,
                    "protocol": db_server.protocol,
                    "address": db_server.address
                }))
            },
            Err(e) => {
                FunctionResult::success(json!({
                    "id": id,
                    "name": db_server.name,
                    "status": "Offline",
                    "error": e.to_string(),
                    "protocol": db_server.protocol
                }))
            }
        }
    }
}

pub struct CompareUpstreamResponsesFunction;

#[async_trait]
impl LlmFunction for CompareUpstreamResponsesFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "compare_upstream_responses".to_string(),
            description: "对比多个上游服务器的解析结果".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "domain": {"type": "string"},
                    "record_type": {"type": "string"},
                    "upstream_ids": {"type": "array", "items": {"type": "integer"}}
                },
                "required": ["domain", "upstream_ids"]
            }),
        }
    }

    async fn execute(&self, args: Value, state: &AppState) -> FunctionResult {
        let domain = args.get("domain").and_then(|v| v.as_str()).unwrap_or("");
        let record_type_str = args.get("record_type").and_then(|v| v.as_str()).unwrap_or("A");
        let ids: Vec<i64> = args.get("upstream_ids")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_i64()).collect())
            .unwrap_or_default();

        if domain.is_empty() || ids.is_empty() {
             return FunctionResult::error("Domain and upstream_ids are required");
        }

        let record_type: RecordType = match RecordType::from_str(record_type_str) {
            Ok(rt) => rt,
            Err(_) => return FunctionResult::error("Invalid record type"),
        };

        let mut results = Vec::new();
        let query = DnsQuery::new(domain, record_type);

        for id in ids {
            if let Ok(Some(db_server)) = state.db.upstream_servers().get_by_id(id).await {
                if let Some(protocol) = UpstreamProtocol::from_str(&db_server.protocol) {
                    let server_config = UpstreamServer::new(
                        db_server.id,
                        db_server.name.clone(),
                        db_server.address.clone(),
                        protocol,
                        db_server.timeout as u32,
                    );
                    
                    let client: Box<dyn DnsClient> = match protocol {
                        UpstreamProtocol::Udp => Box::new(UdpDnsClient::new(server_config)),
                        UpstreamProtocol::Dot => Box::new(DotDnsClient::new(server_config)),
                        UpstreamProtocol::Doh => Box::new(DohDnsClient::new(server_config)),
                        UpstreamProtocol::Doq => Box::new(DoqDnsClient::new(server_config)),
                        UpstreamProtocol::Doh3 => Box::new(Doh3DnsClient::new(server_config)),
                    };

                    let start = Instant::now();
                    let res = match client.query(&query).await {
                        Ok(r) => json!({
                            "status": "Success",
                            "answers": r.response.answers.iter().map(|a| a.value.clone()).collect::<Vec<_>>(),
                            "latency_ms": start.elapsed().as_millis()
                        }),
                        Err(e) => json!({
                            "status": "Error",
                            "error": e.to_string()
                        })
                    };
                    
                    results.push(json!({
                        "server": db_server.name,
                        "result": res
                    }));
                }
            }
        }

        FunctionResult::success(json!({
            "domain": domain,
            "type": record_type_str,
            "comparisons": results
        }))
    }
}

/// List all upstream servers
pub struct ListUpstreamServersFunction;

#[async_trait]
impl LlmFunction for ListUpstreamServersFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "list_upstream_servers".to_string(),
            description: "列出所有上游服务器及其状态".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "enabled_only": {
                        "type": "boolean", 
                        "description": "是否只显示已启用的服务器，默认 true"
                    }
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, args: Value, state: &AppState) -> FunctionResult {
        let enabled_only = args.get("enabled_only").and_then(|v| v.as_bool()).unwrap_or(true);
        
        let servers = if enabled_only {
            match state.db.upstream_servers().list_enabled().await {
                Ok(s) => s,
                Err(e) => return FunctionResult::error(format!("Database error: {}", e)),
            }
        } else {
            match state.db.upstream_servers().list().await {
                Ok(s) => s,
                Err(e) => return FunctionResult::error(format!("Database error: {}", e)),
            }
        };

        let server_list: Vec<Value> = servers.iter().map(|s| json!({
            "id": s.id,
            "name": s.name,
            "address": s.address,
            "protocol": s.protocol,
            "timeout_ms": s.timeout,
            "enabled": s.enabled
        })).collect();

        FunctionResult::success(json!({
            "total": server_list.len(),
            "servers": server_list
        }))
    }
}

/// Query DNS using a specific or random upstream server
pub struct QuerySingleUpstreamFunction;

#[async_trait]
impl LlmFunction for QuerySingleUpstreamFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "query_single_upstream".to_string(),
            description: "使用指定或随机的上游服务器解析域名".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "domain": {"type": "string", "description": "要解析的域名"},
                    "record_type": {"type": "string", "enum": ["A", "AAAA", "CNAME", "MX", "TXT", "NS"]},
                    "upstream_id": {"type": "integer", "description": "指定上游服务器 ID（可选，不填则随机选择）"}
                },
                "required": ["domain"]
            }),
        }
    }

    async fn execute(&self, args: Value, state: &AppState) -> FunctionResult {
        let domain = args.get("domain").and_then(|v| v.as_str()).unwrap_or("");
        let record_type_str = args.get("record_type").and_then(|v| v.as_str()).unwrap_or("A");
        let upstream_id = args.get("upstream_id").and_then(|v| v.as_i64());

        if domain.is_empty() {
            return FunctionResult::error("域名不能为空");
        }

        let record_type: RecordType = match RecordType::from_str(record_type_str) {
            Ok(rt) => rt,
            Err(_) => return FunctionResult::error("Invalid record type"),
        };

        // Get the target server
        let db_server = if let Some(id) = upstream_id {
            // Use specified server
            match state.db.upstream_servers().get_by_id(id).await {
                Ok(Some(s)) => s,
                Ok(None) => return FunctionResult::error(format!("未找到 ID 为 {} 的上游服务器", id)),
                Err(e) => return FunctionResult::error(format!("Database error: {}", e)),
            }
        } else {
            // Random selection
            let servers = match state.db.upstream_servers().list_enabled().await {
                Ok(s) if !s.is_empty() => s,
                Ok(_) => return FunctionResult::error("没有启用的上游服务器"),
                Err(e) => return FunctionResult::error(format!("Database error: {}", e)),
            };
            
            use std::time::{SystemTime, UNIX_EPOCH};
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos() as usize;
            servers[nanos % servers.len()].clone()
        };

        let protocol = match UpstreamProtocol::from_str(&db_server.protocol) {
            Some(p) => p,
            None => return FunctionResult::error(format!("Invalid protocol: {}", db_server.protocol)),
        };

        let server_config = UpstreamServer::new(
            db_server.id,
            db_server.name.clone(),
            db_server.address.clone(),
            protocol,
            db_server.timeout as u32,
        );

        let client: Box<dyn DnsClient> = match protocol {
            UpstreamProtocol::Udp => Box::new(UdpDnsClient::new(server_config)),
            UpstreamProtocol::Dot => Box::new(DotDnsClient::new(server_config)),
            UpstreamProtocol::Doh => Box::new(DohDnsClient::new(server_config)),
            UpstreamProtocol::Doq => Box::new(DoqDnsClient::new(server_config)),
            UpstreamProtocol::Doh3 => Box::new(Doh3DnsClient::new(server_config)),
        };

        let query = DnsQuery::new(domain, record_type);
        let start = Instant::now();
        
        match client.query(&query).await {
            Ok(r) => {
                let answers: Vec<String> = r.response.answers.iter().map(|a| a.value.clone()).collect();
                FunctionResult::success(json!({
                    "domain": domain,
                    "type": record_type_str,
                    "server": {
                        "id": db_server.id,
                        "name": db_server.name,
                        "protocol": db_server.protocol
                    },
                    "status": "Success",
                    "response_code": r.response.response_code.to_string(),
                    "latency_ms": start.elapsed().as_millis() as u64,
                    "answers": answers
                }))
            },
            Err(e) => {
                FunctionResult::success(json!({
                    "domain": domain,
                    "type": record_type_str,
                    "server": {
                        "id": db_server.id,
                        "name": db_server.name,
                        "protocol": db_server.protocol
                    },
                    "status": "Failed",
                    "error": e.to_string(),
                    "latency_ms": start.elapsed().as_millis() as u64
                }))
            }
        }
    }
}

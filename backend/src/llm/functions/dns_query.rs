// DNS Query Function - Execute DNS lookups

use async_trait::async_trait;
use serde_json::{json, Value};

use super::LlmFunction;
use crate::llm::types::{FunctionDefinition, FunctionResult};
use crate::state::AppState;

/// Query DNS records
pub struct QueryDnsFunction;

#[async_trait]
impl LlmFunction for QueryDnsFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "query_dns".to_string(),
            description: "查询域名解析，支持所有记录类型。反向解析（IP → 域名）使用 PTR 类型".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "要查询的域名或 IP 地址（PTR 查询时）"
                    },
                    "record_type": {
                        "type": "string",
                        "description": "记录类型",
                        "enum": ["A", "AAAA", "CNAME", "MX", "TXT", "PTR", "NS", "SOA", "SRV", "CAA"]
                    }
                },
                "required": ["name", "record_type"]
            }),
        }
    }

    async fn execute(&self, args: Value, _state: &AppState) -> FunctionResult {
        let name = match args.get("name").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => return FunctionResult::error("Missing required parameter: name"),
        };
        
        let record_type = args.get("record_type")
            .and_then(|v| v.as_str())
            .unwrap_or("A");

        // For PTR queries, convert IP to reverse DNS format
        let query_name = if record_type == "PTR" {
            convert_ip_to_ptr(name)
        } else {
            name.to_string()
        };

        // TODO: Integrate with actual DNS resolver (ProxyManager)
        // For now, return a placeholder response
        FunctionResult::success(json!({
            "query": {
                "name": query_name,
                "type": record_type,
                "original_name": name
            },
            "message": "DNS 查询功能待集成 ProxyManager",
            "note": "实际查询将通过配置的上游服务器执行"
        }))
    }
}

/// Convert IP address to PTR query format
fn convert_ip_to_ptr(ip: &str) -> String {
    if ip.contains(':') {
        // IPv6
        // TODO: Implement IPv6 reverse DNS format
        format!("{}.ip6.arpa", ip)
    } else if ip.contains('.') {
        // IPv4: reverse the octets and append .in-addr.arpa
        let octets: Vec<&str> = ip.split('.').collect();
        if octets.len() == 4 {
            format!("{}.{}.{}.{}.in-addr.arpa", 
                    octets[3], octets[2], octets[1], octets[0])
        } else {
            ip.to_string()
        }
    } else {
        ip.to_string()
    }
}

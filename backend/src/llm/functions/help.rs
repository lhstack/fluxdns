// Help Functions - Provide help and explanations to users

use async_trait::async_trait;
use serde_json::{json, Value};

use super::LlmFunction;
use crate::llm::types::{FunctionDefinition, FunctionResult};
use crate::state::AppState;

/// Get help information about available commands
pub struct GetHelpFunction;

#[async_trait]
impl LlmFunction for GetHelpFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "get_help".to_string(),
            description: "获取可用命令的帮助信息。可以指定主题获取详细帮助。".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "description": "帮助主题，可选值：dns_records, rewrite_rules, upstreams, dns_query, logs, settings, cache, listeners, diagnostics, analytics, config",
                        "enum": ["dns_records", "rewrite_rules", "upstreams", "dns_query", "logs", "settings", "cache", "listeners", "diagnostics", "analytics", "config"]
                    }
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, args: Value, _state: &AppState) -> FunctionResult {
        let topic = args.get("topic").and_then(|v| v.as_str());

        let help_content = match topic {
            Some("dns_records") => json!({
                "topic": "DNS 记录管理",
                "description": "管理本地 DNS 解析记录",
                "functions": [
                    {"name": "batch_add_dns_records", "description": "批量添加 DNS 记录"},
                    {"name": "edit_dns_record", "description": "编辑单条 DNS 记录"},
                    {"name": "delete_dns_record", "description": "删除 DNS 记录"},
                    {"name": "list_dns_records", "description": "列出所有 DNS 记录"}
                ]
            }),
            Some("rewrite_rules") => json!({
                "topic": "重写规则管理",
                "description": "管理 DNS 查询重写规则，支持精确匹配、通配符和正则表达式",
                "functions": [
                    {"name": "batch_add_rewrite_rules", "description": "批量添加重写规则"},
                    {"name": "edit_rewrite_rule", "description": "编辑单条规则"},
                    {"name": "delete_rewrite_rule", "description": "删除规则"},
                    {"name": "list_rewrite_rules", "description": "列出所有规则"}
                ]
            }),
            Some("upstreams") => json!({
                "topic": "上游服务器管理",
                "description": "管理上游 DNS 服务器，支持多种协议（UDP/TCP/DoT/DoH/DoQ/DoH3）",
                "functions": [
                    {"name": "batch_import_upstreams", "description": "批量导入上游服务器"},
                    {"name": "edit_upstream", "description": "编辑上游服务器"},
                    {"name": "delete_upstream", "description": "删除上游服务器"},
                    {"name": "check_upstream_health", "description": "检测单个上游健康状态"},
                    {"name": "batch_check_upstreams", "description": "批量检测所有上游"},
                    {"name": "reset_upstream_health", "description": "恢复不健康的上游"},
                    {"name": "analyze_upstreams", "description": "分析上游性能"}
                ]
            }),
            Some("dns_query") => json!({
                "topic": "DNS 查询",
                "description": "执行 DNS 查询，支持所有记录类型",
                "functions": [
                    {"name": "query_dns", "description": "查询域名解析，支持 A/AAAA/CNAME/MX/TXT/PTR/NS/SOA/SRV/CAA 等类型"}
                ],
                "tips": [
                    "使用 PTR 类型可以进行反向解析（IP → 域名）",
                    "系统会自动将 IP 地址转换为 PTR 查询格式"
                ]
            }),
            Some("logs") => json!({
                "topic": "日志分析",
                "description": "分析 DNS 查询日志",
                "functions": [
                    {"name": "analyze_query_logs", "description": "综合分析查询日志"},
                    {"name": "get_high_frequency_queries", "description": "获取高频查询排行"},
                    {"name": "detect_anomalies", "description": "检测异常流量"},
                    {"name": "get_domain_query_count", "description": "统计指定域名的查询次数"},
                    {"name": "get_query_ranking", "description": "获取域名查询排名"}
                ]
            }),
            Some("settings") => json!({
                "topic": "系统设置",
                "description": "管理系统配置和日志清理",
                "functions": [
                    {"name": "get_system_status", "description": "获取系统运行状态"},
                    {"name": "update_query_strategy", "description": "更新查询策略"},
                    {"name": "toggle_record_types", "description": "切换记录类型开关"},
                    {"name": "clear_cache", "description": "清空缓存"},
                    {"name": "get_log_retention_settings", "description": "获取日志保留设置"},
                    {"name": "update_log_retention_settings", "description": "更新日志保留设置"},
                    {"name": "cleanup_logs_before_date", "description": "清理指定日期前的日志"},
                    {"name": "cleanup_all_logs", "description": "清空所有日志"}
                ]
            }),
            Some("cache") => json!({
                "topic": "缓存管理",
                "description": "管理 DNS 缓存",
                "functions": [
                    {"name": "get_cache_stats", "description": "获取缓存统计信息"},
                    {"name": "lookup_cache_entry", "description": "查询缓存中的条目"},
                    {"name": "delete_cache_entry", "description": "删除指定缓存条目"}
                ]
            }),
            Some("listeners") => json!({
                "topic": "监听器管理",
                "description": "管理 DNS 服务监听器",
                "functions": [
                    {"name": "list_listeners", "description": "列出所有监听器"},
                    {"name": "add_listener", "description": "添加新监听器"},
                    {"name": "edit_listener", "description": "编辑监听器"},
                    {"name": "delete_listener", "description": "删除监听器"}
                ]
            }),
            Some("diagnostics") => json!({
                "topic": "诊断工具",
                "description": "DNS 解析诊断和测试",
                "functions": [
                    {"name": "trace_dns_resolution", "description": "追踪 DNS 解析全过程"},
                    {"name": "test_upstream_connectivity", "description": "测试上游服务器连通性"},
                    {"name": "compare_upstream_responses", "description": "对比多个上游的解析结果"}
                ]
            }),
            Some("analytics") => json!({
                "topic": "智能分析",
                "description": "高级数据分析和安全检测",
                "functions": [
                    {"name": "get_client_stats", "description": "获取客户端查询统计"},
                    {"name": "detect_dns_tunneling", "description": "检测 DNS 隧道攻击"},
                    {"name": "suggest_blocking_rules", "description": "智能推荐阻止规则"}
                ]
            }),
            Some("config") => json!({
                "topic": "配置管理",
                "description": "导入导出配置和备份",
                "functions": [
                    {"name": "export_config", "description": "导出当前配置"},
                    {"name": "import_config", "description": "导入配置"},
                    {"name": "backup_database", "description": "备份数据库"}
                ]
            }),
            None | Some(_) => json!({
                "description": "FluxDNS AI 助手可以帮助你管理 DNS 服务",
                "available_topics": [
                    {"name": "dns_records", "description": "DNS 记录管理"},
                    {"name": "rewrite_rules", "description": "重写规则管理"},
                    {"name": "upstreams", "description": "上游服务器管理"},
                    {"name": "dns_query", "description": "DNS 查询"},
                    {"name": "logs", "description": "日志分析"},
                    {"name": "settings", "description": "系统设置"},
                    {"name": "cache", "description": "缓存管理"},
                    {"name": "listeners", "description": "监听器管理"},
                    {"name": "diagnostics", "description": "诊断工具"},
                    {"name": "analytics", "description": "智能分析"},
                    {"name": "config", "description": "配置管理"}
                ],
                "tips": [
                    "你可以用自然语言告诉我你想做什么",
                    "例如：'帮我添加一条 A 记录，将 test.com 指向 1.2.3.4'",
                    "或者：'分析一下最近的查询日志，有没有异常'"
                ]
            }),
        };

        FunctionResult::success(help_content)
    }
}

/// Explain DNS record types
pub struct ExplainRecordTypeFunction;

#[async_trait]
impl LlmFunction for ExplainRecordTypeFunction {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: "explain_record_type".to_string(),
            description: "解释 DNS 记录类型的用途和格式".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "type": {
                        "type": "string",
                        "description": "要解释的 DNS 记录类型",
                        "enum": ["A", "AAAA", "CNAME", "MX", "TXT", "PTR", "NS", "SOA", "SRV", "CAA"]
                    }
                },
                "required": ["type"]
            }),
        }
    }

    async fn execute(&self, args: Value, _state: &AppState) -> FunctionResult {
        let record_type = match args.get("type").and_then(|v| v.as_str()) {
            Some(t) => t.to_uppercase(),
            None => return FunctionResult::error("Missing required parameter: type"),
        };

        let explanation = match record_type.as_str() {
            "A" => json!({
                "type": "A",
                "name": "Address Record",
                "description": "将域名映射到 IPv4 地址",
                "format": "IPv4 地址，如 192.168.1.1",
                "example": {"name": "example.com", "value": "93.184.216.34"},
                "use_cases": ["网站托管", "服务器指向", "负载均衡"]
            }),
            "AAAA" => json!({
                "type": "AAAA",
                "name": "IPv6 Address Record",
                "description": "将域名映射到 IPv6 地址",
                "format": "IPv6 地址，如 2001:db8::1",
                "example": {"name": "example.com", "value": "2606:2800:220:1:248:1893:25c8:1946"},
                "use_cases": ["IPv6 网络支持", "双栈部署"]
            }),
            "CNAME" => json!({
                "type": "CNAME",
                "name": "Canonical Name Record",
                "description": "将域名别名指向另一个域名",
                "format": "目标域名（必须是 FQDN）",
                "example": {"name": "www.example.com", "value": "example.com"},
                "use_cases": ["域名别名", "CDN 配置", "子域名重定向"],
                "notes": ["不能与其他记录类型共存", "不能用于根域名"]
            }),
            "MX" => json!({
                "type": "MX",
                "name": "Mail Exchange Record",
                "description": "指定处理该域名邮件的邮件服务器",
                "format": "优先级 + 邮件服务器域名",
                "example": {"name": "example.com", "priority": 10, "value": "mail.example.com"},
                "use_cases": ["邮件路由", "多邮件服务器配置"],
                "notes": ["优先级越低越优先", "可以配置多条实现备份"]
            }),
            "TXT" => json!({
                "type": "TXT",
                "name": "Text Record",
                "description": "存储任意文本信息",
                "format": "任意文本字符串",
                "example": {"name": "example.com", "value": "v=spf1 include:_spf.google.com ~all"},
                "use_cases": ["SPF 邮件验证", "DKIM 签名", "域名所有权验证", "DMARC 策略"],
                "notes": ["单条记录最大 255 字符", "可拆分为多个字符串"]
            }),
            "PTR" => json!({
                "type": "PTR",
                "name": "Pointer Record",
                "description": "反向 DNS 查询，将 IP 地址映射回域名",
                "format": "反向 IP 地址格式",
                "example": {"name": "34.216.184.93.in-addr.arpa", "value": "example.com"},
                "use_cases": ["邮件服务器验证", "反垃圾邮件检查", "网络诊断"],
                "notes": ["IPv4 使用 in-addr.arpa 后缀", "IPv6 使用 ip6.arpa 后缀"]
            }),
            "NS" => json!({
                "type": "NS",
                "name": "Name Server Record",
                "description": "指定该域名的权威 DNS 服务器",
                "format": "DNS 服务器域名",
                "example": {"name": "example.com", "value": "ns1.example.com"},
                "use_cases": ["域名委派", "子域名服务器指定"],
                "notes": ["通常需要配置多个 NS 记录"]
            }),
            "SOA" => json!({
                "type": "SOA",
                "name": "Start of Authority Record",
                "description": "包含域名的管理信息",
                "format": "主 NS、管理员邮箱、序列号、刷新间隔等",
                "fields": ["Primary NS", "Admin Email", "Serial", "Refresh", "Retry", "Expire", "Minimum TTL"],
                "use_cases": ["区域传输", "缓存控制"],
                "notes": ["每个区域只能有一条 SOA 记录"]
            }),
            "SRV" => json!({
                "type": "SRV",
                "name": "Service Record",
                "description": "指定特定服务的位置信息",
                "format": "优先级、权重、端口、目标服务器",
                "example": {"name": "_sip._tcp.example.com", "priority": 10, "weight": 5, "port": 5060, "value": "sipserver.example.com"},
                "use_cases": ["VoIP 服务发现", "即时通讯服务", "LDAP 服务"],
                "notes": ["服务名格式：_service._protocol.domain"]
            }),
            "CAA" => json!({
                "type": "CAA",
                "name": "Certification Authority Authorization",
                "description": "指定哪些 CA 可以为该域名颁发证书",
                "format": "flag tag value",
                "example": {"name": "example.com", "flag": 0, "tag": "issue", "value": "letsencrypt.org"},
                "use_cases": ["SSL/TLS 证书控制", "安全增强"],
                "notes": ["常用 tag: issue, issuewild, iodef"]
            }),
            _ => json!({
                "error": format!("未知的记录类型: {}", record_type),
                "supported_types": ["A", "AAAA", "CNAME", "MX", "TXT", "PTR", "NS", "SOA", "SRV", "CAA"]
            }),
        };

        FunctionResult::success(explanation)
    }
}

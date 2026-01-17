// Function Registry - Central registry for all LLM-callable functions
//
// This module provides the infrastructure for registering and executing
// functions that can be called by the LLM.

pub mod dns_records;
pub mod rewrite_rules;
pub mod upstreams;
pub mod dns_query;
pub mod logs;
pub mod settings;
pub mod cache;
pub mod listeners;
pub mod diagnostics;
pub mod analytics;
pub mod config_mgmt;
pub mod help;

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;

use super::types::{FunctionDefinition, FunctionResult, ToolDefinition};
use crate::state::AppState;

/// Trait for implementing callable functions
#[async_trait]
pub trait LlmFunction: Send + Sync {
    /// Get the function definition for the LLM
    fn definition(&self) -> FunctionDefinition;
    
    /// Execute the function with the given arguments
    async fn execute(&self, args: Value, state: &AppState) -> FunctionResult;
}

/// Central registry for all LLM-callable functions
pub struct FunctionRegistry {
    functions: HashMap<String, Arc<dyn LlmFunction>>,
    state: Arc<AppState>,
}

impl FunctionRegistry {
    /// Create a new function registry with the given app state
    pub fn new(state: Arc<AppState>) -> Self {
        let mut registry = Self {
            functions: HashMap::new(),
            state,
        };
        
        // Register all functions
        registry.register_all();
        
        registry
    }

    /// Register all available functions
    fn register_all(&mut self) {
        // Help functions (always available)
        self.register(Arc::new(help::GetHelpFunction));
        self.register(Arc::new(help::ExplainRecordTypeFunction));
        
        // DNS Records functions
        self.register(Arc::new(dns_records::BatchAddDnsRecordsFunction));
        self.register(Arc::new(dns_records::EditDnsRecordFunction));
        self.register(Arc::new(dns_records::DeleteDnsRecordFunction));
        self.register(Arc::new(dns_records::ListDnsRecordsFunction));
        
        // Rewrite Rules functions
        self.register(Arc::new(rewrite_rules::BatchAddRewriteRulesFunction));
        self.register(Arc::new(rewrite_rules::EditRewriteRuleFunction));
        self.register(Arc::new(rewrite_rules::DeleteRewriteRuleFunction));
        self.register(Arc::new(rewrite_rules::ListRewriteRulesFunction));
        
        // Upstream servers functions
        self.register(Arc::new(upstreams::BatchImportUpstreamsFunction));
        self.register(Arc::new(upstreams::EditUpstreamFunction));
        self.register(Arc::new(upstreams::DeleteUpstreamFunction));
        self.register(Arc::new(upstreams::CheckUpstreamHealthFunction));
        self.register(Arc::new(upstreams::BatchCheckUpstreamsFunction));
        self.register(Arc::new(upstreams::ResetUpstreamHealthFunction));
        self.register(Arc::new(upstreams::AnalyzeUpstreamsFunction));
        
        // DNS Query function
        self.register(Arc::new(dns_query::QueryDnsFunction));
        
        // Log analysis functions
        self.register(Arc::new(logs::AnalyzeQueryLogsFunction));
        self.register(Arc::new(logs::GetHighFrequencyQueriesFunction));
        self.register(Arc::new(logs::DetectAnomaliesFunction));
        self.register(Arc::new(logs::GetDomainQueryCountFunction));
        self.register(Arc::new(logs::GetQueryRankingFunction));
        
        // System settings functions
        self.register(Arc::new(settings::GetSystemStatusFunction));
        self.register(Arc::new(settings::UpdateQueryStrategyFunction));
        self.register(Arc::new(settings::ToggleRecordTypesFunction));
        self.register(Arc::new(settings::ClearCacheFunction));
        self.register(Arc::new(settings::GetLogRetentionSettingsFunction));
        self.register(Arc::new(settings::UpdateLogRetentionSettingsFunction));
        self.register(Arc::new(settings::CleanupLogsBeforeDateFunction));
        self.register(Arc::new(settings::CleanupAllLogsFunction));
        
        // Cache management functions
        self.register(Arc::new(cache::GetCacheStatsFunction));
        self.register(Arc::new(cache::LookupCacheEntryFunction));
        self.register(Arc::new(cache::DeleteCacheEntryFunction));
        
        // Listener management functions
        self.register(Arc::new(listeners::ListListenersFunction));
        self.register(Arc::new(listeners::AddListenerFunction));
        self.register(Arc::new(listeners::EditListenerFunction));
        self.register(Arc::new(listeners::DeleteListenerFunction));
        
        // Diagnostics functions
        self.register(Arc::new(diagnostics::TraceDnsResolutionFunction));
        self.register(Arc::new(diagnostics::TestUpstreamConnectivityFunction));
        self.register(Arc::new(diagnostics::CompareUpstreamResponsesFunction));
        self.register(Arc::new(diagnostics::ListUpstreamServersFunction));
        self.register(Arc::new(diagnostics::QuerySingleUpstreamFunction));
        
        // Analytics functions
        self.register(Arc::new(analytics::GetClientStatsFunction));
        self.register(Arc::new(analytics::DetectDnsTunnelingFunction));
        self.register(Arc::new(analytics::SuggestBlockingRulesFunction));
        
        // Config management functions
        self.register(Arc::new(config_mgmt::ExportConfigFunction));
        self.register(Arc::new(config_mgmt::ImportConfigFunction));
        self.register(Arc::new(config_mgmt::BackupDatabaseFunction));
    }

    /// Register a single function
    fn register(&mut self, func: Arc<dyn LlmFunction>) {
        let name = func.definition().name.clone();
        self.functions.insert(name, func);
    }

    /// Get all tool definitions for the LLM
    pub fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        self.functions
            .values()
            .map(|f| ToolDefinition::new(f.definition()))
            .collect()
    }

    /// Get a specific function definition
    #[allow(dead_code)]
    pub fn get_function(&self, name: &str) -> Option<&Arc<dyn LlmFunction>> {
        self.functions.get(name)
    }

    /// Execute a function by name
    pub async fn execute(&self, name: &str, args_json: &str) -> FunctionResult {
        let func = match self.functions.get(name) {
            Some(f) => f,
            None => return FunctionResult::error(format!("Unknown function: {}", name)),
        };

        let args: Value = match serde_json::from_str(args_json) {
            Ok(v) => v,
            Err(e) => return FunctionResult::error(format!("Invalid arguments: {}", e)),
        };

        func.execute(args, &self.state).await
    }

    /// Get the count of registered functions
    #[allow(dead_code)]
    pub fn count(&self) -> usize {
        self.functions.len()
    }

    /// Get all function names
    #[allow(dead_code)]
    pub fn function_names(&self) -> Vec<&str> {
        self.functions.keys().map(|s| s.as_str()).collect()
    }
}

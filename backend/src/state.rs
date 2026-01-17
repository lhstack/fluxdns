use std::sync::Arc;
use crate::config::ConfigManager;
use crate::db::Database;
use crate::log::LogManager;
use crate::dns::{CacheManager, DnsResolver, ProxyManager, RewriteEngine, UpstreamManager};

/// Application state shared across all components
#[allow(dead_code)]
pub struct AppState {
    pub config: Arc<ConfigManager>,
    pub db: Arc<Database>,
    pub log_manager: Arc<LogManager>,
    pub resolver: Arc<DnsResolver>,
    pub cache: Arc<CacheManager>,
    pub proxy: Arc<ProxyManager>,
    pub rewrite_engine: Arc<RewriteEngine>,
    pub upstream_manager: Arc<UpstreamManager>,
}

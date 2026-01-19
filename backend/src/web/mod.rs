//! Web module
//!
//! Contains the Axum web server and REST API implementations.

pub mod auth;
pub mod cache;
pub mod dns_query;
pub mod listeners;
pub mod llm;
pub mod logs;
pub mod records;
pub mod rewrite;
pub mod settings;
pub mod static_files;
pub mod status;
pub mod strategy;
pub mod upstreams;


pub use auth::{
    auth_middleware, ApiError, AuthService, AuthState,
};
pub use cache::{cache_router, CacheState};
pub use dns_query::{dns_query_router, DnsQueryState};
pub use listeners::{listeners_router, ListenersState};
pub use logs::{logs_router, LogsState};
pub use records::{
    records_router, RecordsState,
};
pub use rewrite::{rewrite_router, RewriteState};
pub use settings::{settings_router, SettingsState};
pub use static_files::{fallback_handler, index_handler, static_handler};
pub use status::{status_router, StatusState};
pub use strategy::{strategy_router, StrategyState};
pub use upstreams::{upstreams_router, UpstreamsState};
pub use llm::{llm_router, LlmState};


//! FluxDNS
//!
//! A modern DNS proxy service supporting multiple protocols (UDP, DoT, DoH, DoQ, DoH3)
//! with a web management interface.

mod config;
mod db;
mod dns;
mod error;
mod log;
mod web;

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use tokio::signal;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use crate::config::ConfigManager;
use crate::db::Database;
use crate::dns::{CacheConfig, CacheManager, DnsResolver, ProxyManager, RewriteEngine, UpstreamManager};
use crate::dns::server::{DohDnsServer, UdpDnsServer};
use crate::log::{LogConfig, LogManager};
use crate::web::{
    auth_middleware, cache_router, dns_query_router, fallback_handler, index_handler,
    logs_router, records_router, rewrite_router, static_handler, status_router,
    strategy_router, upstreams_router, AuthService, AuthState, CacheState, DnsQueryState,
    LogsState, RecordsState, RewriteState, StatusState, StrategyState, UpstreamsState,
};

/// Application state shared across all components
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

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration first (needed for log config)
    let config = Arc::new(ConfigManager::load()?);
    let app_config = config.get();

    // Initialize logging with configuration
    let log_config = LogConfig {
        path: app_config.log_path.clone(),
        level: app_config.log_level.clone(),
        max_size: app_config.log_max_size,
        rotation: crate::log::RotationPolicy::Daily,
        retention_days: app_config.log_retention_days,
    };
    LogManager::init_with_config(log_config.clone())?;

    info!("Starting FluxDNS...");
    info!("Configuration loaded");

    // Initialize database
    let db = Arc::new(Database::new(&app_config.database_url).await?);
    info!("Database initialized");

    // Create log manager for cleanup operations
    let log_manager = Arc::new(LogManager::new(log_config));

    // Load cache config from database
    let cache_ttl = match db.system_config().get("cache_default_ttl").await? {
        Some(v) => v.parse().unwrap_or(60),
        None => 60,
    };
    let cache_max_entries = match db.system_config().get("cache_max_entries").await? {
        Some(v) => v.parse().unwrap_or(10000),
        None => 10000,
    };

    // Initialize DNS components
    let cache = Arc::new(CacheManager::with_config(CacheConfig {
        default_ttl: cache_ttl,
        max_entries: cache_max_entries,
    }));
    info!("Cache manager initialized (TTL: {}s, max entries: {})", 
          cache_ttl, cache_max_entries);

    let rewrite_engine = Arc::new(RewriteEngine::with_db(db.clone()));
    rewrite_engine.load_rules().await?;
    info!("Rewrite engine initialized ({} rules loaded)", rewrite_engine.rule_count().await);

    let upstream_manager = Arc::new(UpstreamManager::with_db(db.clone()));
    upstream_manager.load_servers().await?;
    info!("Upstream manager initialized ({} servers loaded)", upstream_manager.server_count().await);

    let proxy = Arc::new(ProxyManager::new(upstream_manager.clone()));

    // Load query strategy from database
    if let Some(strategy_str) = db.system_config().get("query_strategy").await? {
        if let Some(strategy) = crate::dns::proxy::QueryStrategy::from_str(&strategy_str) {
            proxy.set_strategy(strategy).await;
            info!("Query strategy loaded: {}", strategy);
        }
    }

    let resolver = Arc::new(DnsResolver::with_db(
        rewrite_engine.clone(),
        cache.clone(),
        proxy.clone(),
        db.clone(),
    ));
    info!("DNS resolver initialized");

    // Create application state
    let _state = Arc::new(AppState {
        config: config.clone(),
        db: db.clone(),
        log_manager: log_manager.clone(),
        resolver: resolver.clone(),
        cache: cache.clone(),
        proxy: proxy.clone(),
        rewrite_engine: rewrite_engine.clone(),
        upstream_manager: upstream_manager.clone(),
    });

    // Perform initial log cleanup
    match log_manager.cleanup_old_logs() {
        Ok(result) => {
            if result.deleted_files > 0 {
                info!(
                    "Log cleanup: deleted {} files, freed {} bytes",
                    result.deleted_files, result.deleted_bytes
                );
            }
        }
        Err(e) => {
            tracing::warn!("Failed to cleanup old logs: {}", e);
        }
    }

    // Start DNS servers based on database configuration
    let mut handles = Vec::new();

    // Load enabled listeners from database
    let listeners = db.server_listeners().list_enabled().await?;
    
    for listener in &listeners {
        match listener.protocol.as_str() {
            "udp" => {
                let udp_addr: SocketAddr = format!("{}:{}", listener.bind_address, listener.port).parse()?;
                let udp_resolver = resolver.clone();
                handles.push(tokio::spawn(async move {
                    match UdpDnsServer::new(udp_addr, udp_resolver).await {
                        Ok(server) => {
                            info!("UDP DNS server listening on {}", udp_addr);
                            let server = Arc::new(server);
                            if let Err(e) = server.run().await {
                                tracing::error!("UDP DNS server error: {}", e);
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to start UDP DNS server: {}", e);
                        }
                    }
                }));
            }
            // DoT, DoH, DoQ servers would be started here when TLS is configured
            _ => {
                info!("Listener {} configured but not yet implemented in startup", listener.protocol);
            }
        }
    }

    // Start DoH DNS server (integrated with web server)
    let doh_server = DohDnsServer::new(resolver.clone());

    // Build web server router
    let auth_service = AuthService::new(config.clone());
    let auth_state = AuthState {
        auth_service: auth_service.clone(),
    };

    // Create sub-routers (these have their own state types)
    let records_routes = records_router(RecordsState { db: db.clone() });
    let rewrite_routes = rewrite_router(RewriteState {
        db: db.clone(),
        rewrite_engine: rewrite_engine.clone(),
    });
    let upstreams_routes = upstreams_router(UpstreamsState {
        db: db.clone(),
        upstream_manager: upstream_manager.clone(),
    });
    let cache_routes = cache_router(CacheState {
        cache: cache.clone(),
        db: db.clone(),
    });
    let dns_query_routes = dns_query_router(DnsQueryState {
        resolver: resolver.clone(),
    });
    let strategy_routes = strategy_router(StrategyState {
        db: db.clone(),
        proxy_manager: proxy.clone(),
    });
    let logs_routes = logs_router(LogsState { db: db.clone() });
    let status_routes = status_router(StatusState {
        db: db.clone(),
        cache: cache.clone(),
        proxy_manager: proxy.clone(),
        upstream_manager: upstream_manager.clone(),
        start_time: Arc::new(RwLock::new(std::time::Instant::now())),
    });
    let listeners_routes = crate::web::listeners_router(crate::web::ListenersState {
        db: db.clone(),
    });
    let doh_routes = doh_server.router();

    // Create protected API router (requires authentication)
    let protected_api = Router::new()
        .nest("/api/records", records_routes)
        .nest("/api/rewrite", rewrite_routes)
        .nest("/api/upstreams", upstreams_routes)
        .nest("/api/cache", cache_routes)
        .nest("/api/dns", dns_query_routes)
        .nest("/api/strategy", strategy_routes)
        .nest("/api/logs", logs_routes)
        .nest("/api/status", status_routes)
        .nest("/api/listeners", listeners_routes)
        .layer(middleware::from_fn_with_state(auth_state.clone(), auth_middleware));

    // Create login router with AuthState
    let login_router = Router::new()
        .route("/api/auth/login", post(crate::web::auth::login_handler))
        .with_state(auth_state.clone());

    // Combine all API routes
    let api_router = Router::new()
        .merge(login_router)
        .merge(protected_api)
        .merge(doh_routes);  // DoH routes don't require authentication

    // Build main router with static files
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(index_handler))
        .merge(api_router)
        .route("/*path", get(static_handler))
        .fallback(fallback_handler)
        .layer(cors);

    // Start web server
    let web_addr: SocketAddr = format!("0.0.0.0:{}", app_config.web_port).parse()?;
    info!("Web server listening on http://{}", web_addr);
    info!("DoH endpoint available at http://{}/dns-query", web_addr);

    let listener = tokio::net::TcpListener::bind(web_addr).await?;
    
    // Spawn web server
    handles.push(tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("Web server error: {}", e);
        }
    }));

    info!("FluxDNS started successfully");
    info!("  - Web UI: http://0.0.0.0:{}", app_config.web_port);
    info!("  - DoH: http://0.0.0.0:{}/dns-query", app_config.web_port);
    
    // Log enabled listeners
    for listener in &listeners {
        info!("  - {}: {}:{}", listener.protocol.to_uppercase(), listener.bind_address, listener.port);
    }

    // Wait for shutdown signal
    shutdown_signal().await;

    info!("Shutting down FluxDNS...");

    // Abort all server tasks
    for handle in handles {
        handle.abort();
    }

    info!("FluxDNS stopped");
    Ok(())
}

/// Wait for shutdown signal (Ctrl+C or SIGTERM)
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal");
        },
        _ = terminate => {
            info!("Received SIGTERM signal");
        },
    }
}

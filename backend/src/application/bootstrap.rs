use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use tokio::signal;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use crate::application::web::{
    auth_middleware, cache_router, dns_query_router, fallback_handler, index_handler,
    listeners_router, logs_router, records_router, rewrite_router, settings_router, static_handler,
    status_router, strategy_router, upstreams_router, AuthService, AuthState, CacheState,
    DnsQueryState, ListenersState, LogsState, RecordsState, RewriteState, SettingsState,
    StatusState, StrategyState, UpstreamsState,
};
use crate::business::cache_business::CacheBusiness;
use crate::business::dns_query_business::DnsQueryBusiness;
use crate::business::listener_business::ListenerBusiness;
use crate::business::log_business::LogBusiness;
use crate::business::record_business::RecordBusiness;
use crate::business::rewrite_business::RewriteBusiness;
use crate::business::setting_business::SettingBusiness;
use crate::business::status_business::StatusBusiness;
use crate::business::strategy_business::StrategyBusiness;
use crate::business::upstream_business::UpstreamBusiness;
use crate::dns::server::DohDnsServer;
use crate::dns::{
    CacheConfig, CacheManager, DnsResolver, ProxyManager, RewriteEngine, UpstreamManager,
};
use crate::infrastructure::config::ConfigManager;
use crate::infrastructure::listener_manager::ListenerManager;
use crate::infrastructure::log::{LogConfig, LogManager};
use crate::infrastructure::monitor::AlertManager;
use crate::infrastructure::repository::{Database, QueryLogWriter};

pub async fn run() -> Result<()> {
    // Load configuration first (needed for log config)
    let config = Arc::new(ConfigManager::load()?);
    let app_config = config.get();

    // Initialize logging with configuration
    let log_config = LogConfig {
        path: app_config.log_path.clone(),
        level: app_config.log_level.clone(),
        max_size: app_config.log_max_size,
        rotation: crate::infrastructure::log::RotationPolicy::Daily,
        retention_days: app_config.log_retention_days,
    };
    LogManager::init_with_config(log_config.clone())?;

    println!("Starting FluxDNS...");
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
    info!(
        "Cache manager initialized (TTL: {}s, max entries: {})",
        cache_ttl, cache_max_entries
    );

    let rewrite_engine = Arc::new(RewriteEngine::with_db(db.clone()));
    rewrite_engine.load_rules().await?;
    info!(
        "Rewrite engine initialized ({} rules loaded)",
        rewrite_engine.rule_count().await
    );

    let upstream_manager = Arc::new(UpstreamManager::with_db(db.clone()));
    upstream_manager.load_servers().await?;
    info!(
        "Upstream manager initialized ({} servers loaded)",
        upstream_manager.server_count().await
    );

    let proxy = Arc::new(ProxyManager::new(upstream_manager.clone()));

    // Load query strategy from database
    if let Some(strategy_str) = db.system_config().get("query_strategy").await? {
        if let Some(strategy) = crate::dns::proxy::QueryStrategy::from_str(&strategy_str) {
            proxy.set_strategy(strategy).await;
            info!("Query strategy loaded: {}", strategy);
        }
    }

    // Query logs are written by a single batching task; the handle is shared so
    // the status endpoint can report entries dropped under load.
    let query_log_writer = QueryLogWriter::start(db.clone());

    let resolver = Arc::new(DnsResolver::with_db(
        rewrite_engine.clone(),
        cache.clone(),
        proxy.clone(),
        db.clone(),
        query_log_writer.clone(),
    ));

    // Prime the hot-path masks. Failing here is fatal on purpose: starting with
    // empty masks would answer for record types the operator disabled.
    resolver.plane_state().reload(&db).await?;
    info!("DNS resolver initialized");

    // Initialize ListenerManager
    let listener_manager = Arc::new(ListenerManager::new(db.clone(), resolver.clone()));

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

    // Start auto cleanup task for query logs
    let cleanup_db = db.clone();
    handles.push(tokio::spawn(async move {
        // Run cleanup check every hour
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;

            // Check if auto cleanup is enabled
            let config = cleanup_db.system_config();
            let enabled = match config.get("log_auto_cleanup_enabled").await {
                Ok(Some(v)) => v == "true",
                _ => false,
            };

            if !enabled {
                continue;
            }

            // Get retention days
            let retention_days = match config.get("log_retention_days").await {
                Ok(Some(v)) => v.parse::<i64>().unwrap_or(30),
                _ => 30,
            };

            // Perform cleanup
            match cleanup_db.query_logs().delete_old(retention_days).await {
                Ok(deleted) => {
                    if deleted > 0 {
                        info!(
                            "Auto cleanup: deleted {} query logs older than {} days",
                            deleted, retention_days
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!("Auto cleanup failed: {}", e);
                }
            }
        }
    }));

    // Start enabled listeners using manager
    listener_manager.start_all_enabled().await;

    // Start DoH DNS server (integrated with web server)
    let doh_server = DohDnsServer::new(resolver.clone());

    // Build web server router
    let auth_service = AuthService::new(config.clone());
    let auth_state = AuthState {
        auth_service: auth_service.clone(),
    };

    // Compose business layer: each use case gets exactly the collaborators it needs
    let record_business = Arc::new(RecordBusiness::new(
        db.clone(),
        resolver.plane_state().clone(),
    ));
    let rewrite_business = Arc::new(RewriteBusiness::new(db.clone(), rewrite_engine.clone()));
    let upstream_business = Arc::new(UpstreamBusiness::new(db.clone(), upstream_manager.clone()));
    let cache_business = Arc::new(CacheBusiness::new(db.clone(), cache.clone()));
    let dns_query_business = Arc::new(DnsQueryBusiness::new(resolver.clone()));
    let strategy_business = Arc::new(StrategyBusiness::new(db.clone(), proxy.clone()));
    let log_business = Arc::new(LogBusiness::new(db.clone()));
    let setting_business = Arc::new(SettingBusiness::new(
        db.clone(),
        resolver.plane_state().clone(),
    ));
    let listener_business = Arc::new(ListenerBusiness::new(db.clone(), listener_manager.clone()));
    let status_business = Arc::new(StatusBusiness::new(
        db.clone(),
        cache.clone(),
        proxy.clone(),
        upstream_manager.clone(),
        upstream_business.clone(),
        query_log_writer.clone(),
        std::time::Instant::now(),
    ));

    // Create sub-routers (each owns its adapter state)
    let records_routes = records_router(RecordsState {
        business: record_business,
    });
    let rewrite_routes = rewrite_router(RewriteState {
        business: rewrite_business,
    });
    let upstreams_routes = upstreams_router(UpstreamsState {
        business: upstream_business,
    });
    let cache_routes = cache_router(CacheState {
        business: cache_business,
    });
    let dns_query_routes = dns_query_router(DnsQueryState {
        business: dns_query_business,
    });
    let strategy_routes = strategy_router(StrategyState {
        business: strategy_business,
    });
    let logs_routes = logs_router(LogsState {
        business: log_business,
    });
    let status_routes = status_router(StatusState {
        business: status_business,
    });
    let listeners_routes = listeners_router(ListenersState {
        business: listener_business,
    });
    let settings_routes = settings_router(SettingsState {
        business: setting_business,
    });
    let doh_routes = doh_server.router();

    // Start alert monitor with its explicit dependencies
    let alert_manager = Arc::new(AlertManager::new(db.clone(), upstream_manager.clone()));
    handles.push(alert_manager.spawn());

    // Actively probe upstream servers so unhealthy ones can return to rotation
    // without waiting for user traffic to hit them.
    handles.push(upstream_manager.clone().spawn_health_checker());

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
        .nest("/api/settings", settings_routes)
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_middleware,
        ));

    // Create login router with AuthState
    let login_router = Router::new()
        .route(
            "/api/auth/login",
            post(crate::application::web::auth::login_handler),
        )
        .with_state(auth_state.clone());

    // Combine all API routes
    let api_router = Router::new()
        .merge(login_router)
        .merge(protected_api)
        .merge(doh_routes); // DoH routes don't require authentication

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

    // Spawn web server with ConnectInfo for client IP extraction
    handles.push(tokio::spawn(async move {
        if let Err(e) = axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        {
            tracing::error!("Web server error: {}", e);
        }
    }));

    println!("FluxDNS started successfully");
    println!("  - Web UI: http://0.0.0.0:{}", app_config.web_port);
    println!("  - DoH: http://0.0.0.0:{}/dns-query", app_config.web_port);

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

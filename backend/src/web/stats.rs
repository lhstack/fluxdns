//! Stats API module
//!
//! Implements real-time statistics streaming via SSE.

use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
    routing::get,
    Json,
    Router,
};
use futures::stream::Stream;
use futures::StreamExt;
use serde::Serialize;
use tokio::time::interval;
use tokio_stream::wrappers::IntervalStream;

use crate::db::repository::TopStats;
use crate::db::Database;
use crate::dns::CacheManager;
use crate::dns::proxy::UpstreamManager;

/// Stats API state
#[derive(Clone)]
pub struct StatsState {
    pub db: Arc<Database>,
    pub cache: Arc<CacheManager>,
    pub upstream_manager: Arc<UpstreamManager>,
}

/// Real-time stats message sent via SSE
#[derive(Debug, Serialize, Clone)]
pub struct StatsMessage {
    pub timestamp: i64,
    pub qps: f64,
    pub avg_latency_ms: f64,
    pub cache_hit_rate: f64,
    pub total_queries: i64,
    pub healthy_upstreams: usize,
    pub total_upstreams: usize,
    pub upstream_status: Vec<SimpleUpstreamStatus>,
}

#[derive(Debug, Serialize, Clone)]
pub struct SimpleUpstreamStatus {
    pub id: i64,
    pub name: String,
    pub healthy: bool,
    pub latency_ms: u64,
}

/// Stream real-time stats
///
/// GET /api/stats/stream
pub async fn stats_stream(
    State(state): State<StatsState>,
) -> Sse<impl Stream<Item = Result<Event, axum::Error>>> {
    // Pre-fetch upstream servers list (changes infrequently)
    let servers = state.db.upstream_servers().list().await.unwrap_or_default();

    let mut previous_total_queries = 0i64;
    // Initial fetch to set baseline
    if let Ok(stats) = state.db.query_logs().get_stats().await {
        previous_total_queries = stats.total_queries;
    }

    let stream = IntervalStream::new(interval(Duration::from_secs(1)))
        .then(move |_| {
            let state = state.clone();
            let servers = servers.clone();
            async move {
                // 1. Get Query Stats (for QPS and Total) - from memory cache
                let query_stats = state.db.query_logs().get_stats().await.unwrap_or_default();
                let current_total = query_stats.total_queries;
                
                // 2. Get Cache Stats - from memory
                let cache_stats = state.cache.stats().await;
                
                // 3. Get Upstream Stats - from memory (servers list pre-fetched)
                let upstream_stats = state.upstream_manager.get_all_stats().await;
                
                let total_upstreams = servers.len();
                let mut healthy_upstreams = 0;
                let mut total_latency = 0;
                let mut latency_count = 0;
                
                let simple_upstreams: Vec<SimpleUpstreamStatus> = servers.iter().map(|s| {
                    let stats = upstream_stats.get(&s.id);
                    let healthy = stats.map(|st| st.is_healthy()).unwrap_or(s.enabled); // Default to enabled status if no stats
                    if healthy && s.enabled {
                        healthy_upstreams += 1;
                    }
                    
                    let lat = stats.map(|st| st.avg_response_time_ms()).unwrap_or(0);
                    if lat > 0 {
                        total_latency += lat;
                        latency_count += 1;
                    }

                    SimpleUpstreamStatus {
                        id: s.id,
                        name: s.name.clone(),
                        healthy,
                        latency_ms: lat,
                    }
                }).collect();

                let avg_latency = if latency_count > 0 {
                    total_latency as f64 / latency_count as f64
                } else {
                    0.0
                };

                (current_total, cache_stats.hit_rate(), avg_latency, simple_upstreams, healthy_upstreams, total_upstreams)
            }
        })
        .scan(previous_total_queries, |prev_total, (curr_total, hit_rate, avg_latency, upstreams, healthy, total_ups)| {
            let qps = if curr_total >= *prev_total {
                (curr_total - *prev_total) as f64
            } else {
                0.0
            };
            *prev_total = curr_total;
            
            futures::future::ready(Some(StatsMessage {
                timestamp: chrono::Utc::now().timestamp_millis(),
                qps,
                avg_latency_ms: avg_latency,
                cache_hit_rate: hit_rate,
                total_queries: curr_total,
                healthy_upstreams: healthy,
                total_upstreams: total_ups,
                upstream_status: upstreams,
            }))
        })
        .map(|msg| {
            Event::default().json_data(msg).map_err(axum::Error::new)
        });

    Sse::new(stream).keep_alive(KeepAlive::default())
}


/// Build the stats API router
pub fn stats_router(state: StatsState) -> Router {
    Router::new()
        .route("/stream", get(stats_stream))
        .route("/top/domains", get(get_top_domains))
        .route("/top/clients", get(get_top_clients))
        .with_state(state)
}

async fn get_top_domains(State(state): State<StatsState>) -> Result<Json<Vec<TopStats>>, String> {
    let stats = state
        .db
        .query_logs()
        .get_top_domains(10)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Json(stats))
}

async fn get_top_clients(State(state): State<StatsState>) -> Result<Json<Vec<TopStats>>, String> {
    let stats = state
        .db
        .query_logs()
        .get_top_clients(10)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Json(stats))
}


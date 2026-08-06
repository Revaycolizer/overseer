//! Metrics and health endpoints for the headless server.
//!
//! - GET /metrics — Prometheus-formatted metrics
//! - GET /health — Simple JSON health check

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde::Serialize;

use crate::server::ServerState;

/// Global metrics counters.
pub static METRICS: OnceLock<MetricsCollector> = OnceLock::new();

pub fn metrics() -> &'static MetricsCollector {
    METRICS.get_or_init(|| MetricsCollector::new())
}

pub struct MetricsCollector {
    pub requests_total: AtomicU64,
    pub errors_total: AtomicU64,
    pub tools_called: AtomicU64,
    pub messages_sent: AtomicU64,
    pub orchestration_runs: AtomicU64,
}

impl MetricsCollector {
    pub const fn new() -> Self {
        Self {
            requests_total: AtomicU64::new(0),
            errors_total: AtomicU64::new(0),
            tools_called: AtomicU64::new(0),
            messages_sent: AtomicU64::new(0),
            orchestration_runs: AtomicU64::new(0),
        }
    }

    pub fn inc_requests(&self) { self.requests_total.fetch_add(1, Ordering::Relaxed); }
    pub fn inc_errors(&self) { self.errors_total.fetch_add(1, Ordering::Relaxed); }
    pub fn inc_tools(&self) { self.tools_called.fetch_add(1, Ordering::Relaxed); }
    pub fn inc_messages(&self) { self.messages_sent.fetch_add(1, Ordering::Relaxed); }
    pub fn inc_runs(&self) { self.orchestration_runs.fetch_add(1, Ordering::Relaxed); }
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    uptime_secs: u64,
    db_connected: bool,
    ollama_reachable: bool,
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/metrics", get(handle_metrics))
        // `/health` is owned by misc::router(); expose the metrics liveness probe
        // under `/healthz` to avoid an overlapping-route panic at startup.
        .route("/healthz", get(handle_health))
}

async fn handle_metrics() -> String {
    let m = metrics();
    format!(
        r#"# HELP overseer_requests_total Total HTTP requests
# TYPE overseer_requests_total counter
overseer_requests_total {}
# HELP overseer_errors_total Total errors
# TYPE overseer_errors_total counter
overseer_errors_total {}
# HELP overseer_tools_called Total tool invocations
# TYPE overseer_tools_called counter
overseer_tools_called {}
# HELP overseer_messages_sent Total messages sent
# TYPE overseer_messages_sent counter
overseer_messages_sent {}
# HELP overseer_orchestration_runs Total orchestration runs
# TYPE overseer_orchestration_runs counter
overseer_orchestration_runs {}
"#,
        m.requests_total.load(Ordering::Relaxed),
        m.errors_total.load(Ordering::Relaxed),
        m.tools_called.load(Ordering::Relaxed),
        m.messages_sent.load(Ordering::Relaxed),
        m.orchestration_runs.load(Ordering::Relaxed),
    )
}

async fn handle_health(State(state): State<Arc<ServerState>>) -> Json<HealthResponse> {
    let db_ok = sqlx::query("SELECT 1")
        .execute(&state.db.pool)
        .await
        .is_ok();

    let ollama_ok = reqwest::Client::new()
        .get("http://localhost:11434/api/tags")
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);

    Json(HealthResponse {
        status: if db_ok { "healthy".into() } else { "degraded".into() },
        version: env!("CARGO_PKG_VERSION").into(),
        uptime_secs: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        db_connected: db_ok,
        ollama_reachable: ollama_ok,
    })
}

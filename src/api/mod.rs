use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};

use crate::db;
use crate::monitors::{MonitorConfig, MonitorStore};
use crate::probe::ProbeResult;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub monitors: MonitorStore,
    /// In-process fan-out of live status updates to connected SSE clients.
    pub updates: broadcast::Sender<StatusUpdate>,
}

// ── Live status updates (SSE) ─────────────────────────────────────────────────

/// A lightweight, push-on-probe status payload. Built directly from a
/// `ProbeResult` (no extra DB query) and broadcast to connected dashboards.
#[derive(Clone, Debug, Serialize)]
pub struct StatusUpdate {
    pub name: String,
    pub protocol: String,
    pub endpoint: String,
    pub status: String,
    pub response_time_ms: Option<u64>,
    pub failure_reason: Option<String>,
    pub detail: Option<String>,
    pub last_checked_at: String,
}

impl From<&ProbeResult> for StatusUpdate {
    fn from(r: &ProbeResult) -> Self {
        Self {
            name: r.monitor_name.clone(),
            protocol: r.protocol.clone(),
            endpoint: r.endpoint.clone(),
            status: r.status.clone(),
            response_time_ms: r.response_time_ms,
            failure_reason: r.failure_reason.clone(),
            detail: r.detail.clone(),
            last_checked_at: r.checked_at.to_rfc3339(),
        }
    }
}

/// `GET /api/monitors/stream` — Server-Sent Events stream of live status
/// updates. Subscribes to the broadcast channel; lagged-subscriber errors are
/// skipped (the dashboard's poll reconciles), and the stream ends when the
/// client disconnects. `KeepAlive` keeps idle connections from timing out.
pub async fn monitor_stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.updates.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|res| match res {
        Ok(update) => Some(Ok(Event::default()
            .json_data(&update)
            .unwrap_or_else(|_| Event::default().comment("serialize error")))),
        // Lagged: the client fell behind and missed events; skip, the poll reconciles.
        Err(_) => None,
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

// ── Existing read-only endpoints ──────────────────────────────────────────────

#[derive(Serialize)]
pub struct MonitorStatus {
    pub name: String,
    pub protocol: String,
    pub endpoint: String,
    pub status: String,
    pub last_checked_at: Option<String>,
    pub response_time_ms: Option<i64>,
    pub failure_reason: Option<String>,
    pub detail: Option<String>,
    pub uptime_24h: Option<f64>,
}

pub async fn list_monitors(State(state): State<AppState>) -> impl IntoResponse {
    // The dashboard reflects the *configured* monitors (monitors.toml), each joined
    // with its latest probe result via an O(1) index seek. This excludes stale probe
    // history for unconfigured monitors and scales with the monitor count, not the
    // total row count. Configured-but-unprobed monitors show as pending.
    let configured = state.monitors.list().await;
    let mut result = Vec::with_capacity(configured.len());
    for m in &configured {
        let name = m.name().to_string();
        match db::get_latest_status(&state.pool, &name).await {
            Ok(Some(s)) => {
                let uptime_24h = db::get_uptime(&state.pool, &name, 86_400).await.ok().flatten();
                result.push(MonitorStatus {
                    name,
                    protocol: s.protocol,
                    endpoint: s.endpoint,
                    status: s.status,
                    last_checked_at: Some(s.checked_at),
                    response_time_ms: s.response_time_ms,
                    failure_reason: s.failure_reason,
                    detail: s.detail,
                    uptime_24h,
                });
            }
            _ => {
                result.push(MonitorStatus {
                    name,
                    protocol: m.protocol().to_string(),
                    endpoint: m.endpoint(),
                    status: "pending".to_string(),
                    last_checked_at: None,
                    response_time_ms: None,
                    failure_reason: None,
                    detail: None,
                    uptime_24h: None,
                });
            }
        }
    }
    Json(result).into_response()
}

#[derive(Deserialize)]
pub struct HistoryParams {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

pub async fn monitor_history(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(params): Query<HistoryParams>,
) -> impl IntoResponse {
    let exists = db::monitor_exists(&state.pool, &name).await.unwrap_or(false);
    if !exists {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "monitor_not_found", "name": name })),
        ).into_response();
    }
    let limit = params.limit.unwrap_or(100).min(1000);
    match db::get_history(&state.pool, &name, params.from, params.to, limit).await {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "DB error in monitor_history");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "db_error"}))).into_response()
        }
    }
}

#[derive(Serialize)]
pub struct UptimeResponse {
    pub uptime_1h: Option<f64>,
    pub uptime_24h: Option<f64>,
    pub uptime_7d: Option<f64>,
    pub uptime_30d: Option<f64>,
}

pub async fn monitor_uptime(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let exists = db::monitor_exists(&state.pool, &name).await.unwrap_or(false);
    if !exists {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "monitor_not_found", "name": name })),
        ).into_response();
    }
    let (u1h, u24h, u7d, u30d) = tokio::join!(
        db::get_uptime(&state.pool, &name, 3_600),
        db::get_uptime(&state.pool, &name, 86_400),
        db::get_uptime(&state.pool, &name, 604_800),
        db::get_uptime(&state.pool, &name, 2_592_000),
    );
    Json(UptimeResponse {
        uptime_1h: u1h.ok().flatten(),
        uptime_24h: u24h.ok().flatten(),
        uptime_7d: u7d.ok().flatten(),
        uptime_30d: u30d.ok().flatten(),
    }).into_response()
}

#[derive(Deserialize)]
pub struct SeriesParams {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub buckets: Option<i64>,
}

pub async fn monitor_series(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(params): Query<SeriesParams>,
) -> impl IntoResponse {
    let to = params.to.unwrap_or_else(Utc::now);
    let from = params.from.unwrap_or_else(|| to - chrono::Duration::hours(24));
    let buckets = params.buckets.unwrap_or(300).clamp(50, 1000);

    match db::get_series(&state.pool, &name, from, to, buckets).await {
        Ok(series) => Json(series).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "DB error in monitor_series");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "db_error"}))).into_response()
        }
    }
}

// ── Traceroute endpoints ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TraceParams {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

/// `GET /api/monitors/:name/traceroute?from&to` — per-hop aggregates over a range.
pub async fn monitor_traceroute(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(params): Query<TraceParams>,
) -> impl IntoResponse {
    let to = params.to.unwrap_or_else(Utc::now);
    let from = params.from.unwrap_or_else(|| to - chrono::Duration::hours(1));
    match db::get_traceroute_hops(&state.pool, &name, from, to).await {
        Ok(hops) => Json(hops).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "DB error in monitor_traceroute");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "db_error"}))).into_response()
        }
    }
}

/// `GET /api/monitors/:name/traceroute/extent` — the retained data extent so the
/// UI brush knows its bounds. `{ "from": null, "to": null }` when there is no data.
pub async fn monitor_traceroute_extent(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match db::get_traceroute_extent(&state.pool, &name).await {
        Ok(Some((from, to))) => Json(serde_json::json!({ "from": from, "to": to })).into_response(),
        Ok(None) => Json(serde_json::json!({ "from": null, "to": null })).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "DB error in monitor_traceroute_extent");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "db_error"}))).into_response()
        }
    }
}

// ── Monitor CRUD endpoints ────────────────────────────────────────────────────

pub async fn list_monitor_configs(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.monitors.list().await).into_response()
}

pub async fn add_monitor(
    State(state): State<AppState>,
    Json(monitor): Json<MonitorConfig>,
) -> impl IntoResponse {
    let errors = validate_monitor(&monitor, &state.monitors.list().await);
    if !errors.is_empty() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "errors": errors })),
        ).into_response();
    }

    match state.monitors.add(monitor).await {
        Ok(()) => (StatusCode::CREATED, Json(state.monitors.list().await)).into_response(),
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "errors": [e.to_string()] })),
        ).into_response(),
    }
}

pub async fn delete_monitor(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match state.monitors.remove(&name).await {
        Ok(true) => {
            // Purge the monitor's stored probe results so it no longer surfaces in
            // the dashboard's status list (derived from probe_results). Best-effort:
            // the config is already gone, so a purge failure must not fail the request.
            if let Err(e) = db::delete_results(&state.pool, &name).await {
                tracing::warn!(monitor = %name, error = %e, "Failed to purge probe results for deleted monitor");
            }
            Json(state.monitors.list().await).into_response()
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "monitor not found" })),
        ).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ).into_response(),
    }
}

// ── Validation ────────────────────────────────────────────────────────────────

fn validate_monitor(monitor: &MonitorConfig, existing: &[MonitorConfig]) -> Vec<String> {
    let mut errors = Vec::new();

    let name = monitor.name();
    if name.is_empty() {
        errors.push("name is required".into());
    } else if name.len() > 100 {
        errors.push("name must be 100 characters or fewer".into());
    } else if name.contains('/') || name.contains('\\') {
        errors.push("name must not contain / or \\".into());
    } else if existing.iter().any(|m| m.name() == name) {
        errors.push(format!("a monitor named '{}' already exists", name));
    }

    match monitor {
        MonitorConfig::Http(c) => {
            if c.url.is_empty() {
                errors.push("url is required".into());
            } else if !c.url.starts_with("http://") && !c.url.starts_with("https://") {
                errors.push("url must be a valid http or https URL".into());
            }
        }
        MonitorConfig::Tcp(c) => {
            if c.host.is_empty() {
                errors.push("host is required".into());
            }
            if c.port == 0 {
                errors.push("port must be between 1 and 65535".into());
            }
        }
        MonitorConfig::Icmp(c) => {
            if c.host.is_empty() {
                errors.push("host is required".into());
            }
        }
        MonitorConfig::PublicIp(c) => {
            // url is optional; if given it must be http(s).
            if let Some(url) = &c.url {
                if !url.starts_with("http://") && !url.starts_with("https://") {
                    errors.push("url must be a valid http or https URL".into());
                }
            }
        }
        MonitorConfig::Border(c) => {
            if let Some(gw) = &c.gateway {
                if !is_valid_host(gw) {
                    errors.push("gateway must be a valid IP or host".into());
                }
            }
            if !is_valid_host(&c.upstream) {
                errors.push("upstream must be a valid IP or host".into());
            }
        }
        MonitorConfig::Traceroute(c) => {
            if c.host.is_empty() {
                errors.push("host is required".into());
            } else if !is_valid_host(&c.host) {
                errors.push("host must be a valid IP or host".into());
            }
        }
    }

    errors
}

/// A pragmatic IP-or-hostname check: a parseable IP, or a non-empty token with no
/// whitespace/control characters and only host-legal characters.
fn is_valid_host(s: &str) -> bool {
    if s.parse::<std::net::IpAddr>().is_ok() {
        return true;
    }
    !s.is_empty()
        && !s.chars().any(|c| c.is_whitespace() || c.is_control())
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == ':')
}

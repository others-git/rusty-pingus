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
use tokio_util::sync::CancellationToken;

use crate::db;
use crate::monitors::{MonitorConfig, MonitorStore};
use crate::probe::ProbeResult;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub monitors: MonitorStore,
    /// In-process fan-out of live status updates to connected SSE clients.
    pub updates: broadcast::Sender<StatusUpdate>,
    /// Global retention in days (from config.toml), used as the fallback when a
    /// monitor has no per-monitor `retention_hours`.
    pub global_retention_days: u64,
    /// Cancelled on shutdown; SSE streams subscribe so they close promptly.
    pub cancel: CancellationToken,
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
    let cancel = state.cancel.clone();
    let inner = BroadcastStream::new(rx).filter_map(|res| match res {
        Ok(update) => Some(Ok(Event::default()
            .json_data(&update)
            .unwrap_or_else(|_| Event::default().comment("serialize error")))),
        // Lagged: the client fell behind and missed events; skip, the poll reconciles.
        Err(_) => None,
    });
    // End the stream (and close the connection) when the app shuts down, so
    // axum's graceful shutdown doesn't wait on long-lived SSE connections.
    let stream = futures_util::StreamExt::take_until(inner, cancel.cancelled_owned());
    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// A monitor's protocol + endpoint from the current config, or neutral fallbacks
/// when it is no longer configured (e.g. history for a since-deleted monitor).
/// These are no longer stored per probe row, so responses source them here.
async fn monitor_identity(state: &AppState, name: &str) -> (String, String) {
    state.monitors.list().await.iter()
        .find(|m| m.name() == name)
        .map(|m| (m.protocol().to_string(), m.endpoint()))
        .unwrap_or_else(|| ("unknown".to_string(), String::new()))
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
    pub enabled: bool,
    /// Per-monitor retention in hours. None means the global default applies
    /// (the frontend should fall back to `global_retention_days × 24`).
    pub retention_hours: Option<u64>,
}

pub async fn list_monitors(State(state): State<AppState>) -> impl IntoResponse {
    // The dashboard reflects the *configured* monitors (monitors.toml), each joined
    // with its latest probe result via an O(1) index seek. This excludes stale probe
    // history for unconfigured monitors and scales with the monitor count, not the
    // total row count. Configured-but-unprobed monitors show as pending.
    let configured = state.monitors.list().await;
    // Batched: one query for every monitor's latest status, one for 24h uptime —
    // instead of two queries per monitor. protocol/endpoint come from the config
    // (no longer stored per probe row).
    let latest: std::collections::HashMap<String, db::CurrentStatus> = db::get_current_status(&state.pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|s| (s.monitor_name.clone(), s))
        .collect();
    let uptime = db::get_uptime_24h_all(&state.pool).await.unwrap_or_default();

    let mut result = Vec::with_capacity(configured.len());
    for m in &configured {
        let name = m.name().to_string();
        let protocol = m.protocol().to_string();
        let endpoint = m.endpoint();
        let enabled = m.enabled();
        // Raw per-monitor retention (None → global default applies at the frontend).
        let retention_hours = match m {
            crate::monitors::MonitorConfig::Http(c) => c.retention_hours,
            crate::monitors::MonitorConfig::Tcp(c) => c.retention_hours,
            crate::monitors::MonitorConfig::Icmp(c) => c.retention_hours,
            crate::monitors::MonitorConfig::PublicIp(c) => c.retention_hours,
            crate::monitors::MonitorConfig::Border(c) => c.retention_hours,
            crate::monitors::MonitorConfig::Traceroute(c) => c.retention_hours,
        };
        match latest.get(&name) {
            Some(s) => result.push(MonitorStatus {
                uptime_24h: uptime.get(&name).copied(),
                protocol,
                endpoint,
                enabled,
                retention_hours,
                status: s.status.clone(),
                last_checked_at: Some(s.checked_at.clone()),
                response_time_ms: s.response_time_ms,
                failure_reason: s.failure_reason.clone(),
                detail: s.detail.clone(),
                name,
            }),
            None => result.push(MonitorStatus {
                name,
                protocol,
                endpoint,
                enabled,
                retention_hours,
                status: "pending".to_string(),
                last_checked_at: None,
                response_time_ms: None,
                failure_reason: None,
                detail: None,
                uptime_24h: None,
            }),
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
        Ok(mut rows) => {
            // protocol/endpoint are not stored per row; supply them from config
            // (the detail page relies on `protocol` to choose its view).
            let (protocol, endpoint) = monitor_identity(&state, &name).await;
            for r in &mut rows { r.protocol = protocol.clone(); r.endpoint = endpoint.clone(); }
            Json(rows).into_response()
        }
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

// ── State-timeline segments + probe extent ────────────────────────────────────

#[derive(Deserialize)]
pub struct SegmentsParams {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

/// `GET /api/monitors/:name/segments?from&to` — collapsed state segments for
/// public-IP/border timelines, server-side, so the full window is covered
/// regardless of probe frequency (bounded by changes, not probe count).
pub async fn monitor_segments(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(params): Query<SegmentsParams>,
) -> impl IntoResponse {
    let to = params.to.unwrap_or_else(Utc::now);
    let from = params.from.unwrap_or_else(|| to - chrono::Duration::hours(24));
    match db::get_state_segments(&state.pool, &name, from.timestamp_millis(), to.timestamp_millis()).await {
        Ok(segs) => Json(segs).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "DB error in monitor_segments");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "db_error"}))).into_response()
        }
    }
}

/// `GET /api/monitors/:name/extent` — earliest/latest probe times and the
/// monitor's effective retention (hours), so the detail-page brush knows its
/// bounds. Works for all monitor types (use traceroute/extent for traceroute).
pub async fn monitor_extent(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let retention_hours = state.monitors.list().await.iter()
        .find(|m| m.name() == name)
        .map(|m| m.retention_hours(state.global_retention_days))
        .unwrap_or(state.global_retention_days.saturating_mul(24));

    let ms_to_iso = |ms: i64| {
        DateTime::<Utc>::from_timestamp_millis(ms).unwrap_or_default().to_rfc3339()
    };

    match db::get_probe_extent(&state.pool, &name).await {
        Ok(Some((lo, hi))) => Json(serde_json::json!({
            "from": ms_to_iso(lo),
            "to": ms_to_iso(hi),
            "retention_hours": retention_hours,
        })).into_response(),
        Ok(None) => Json(serde_json::json!({
            "from": null,
            "to": null,
            "retention_hours": retention_hours,
        })).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "DB error in monitor_extent");
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

#[derive(Deserialize)]
pub struct EnabledBody {
    pub enabled: bool,
}

/// `POST /api/monitors/:name/enabled` — enable or disable a monitor without
/// deleting it. Persists + notifies (the scheduler reacts via hot-reload).
pub async fn set_monitor_enabled(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(body): Json<EnabledBody>,
) -> impl IntoResponse {
    match state.monitors.set_enabled(&name, body.enabled).await {
        Ok(true) => Json(state.monitors.list().await).into_response(),
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
            } else if !is_http_url(&c.url) {
                errors.push("url must be a valid http or https URL".into());
            }
        }
        MonitorConfig::Tcp(c) => {
            require_host(&c.host, &mut errors);
            if c.port == 0 {
                errors.push("port must be between 1 and 65535".into());
            }
        }
        MonitorConfig::Icmp(c) => {
            require_host(&c.host, &mut errors);
        }
        MonitorConfig::PublicIp(c) => {
            // url is optional; if given it must be http(s).
            if let Some(url) = &c.url {
                if !is_http_url(url) {
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
                require_host(&c.host, &mut errors);
            } else if !is_valid_host(&c.host) {
                errors.push("host must be a valid IP or host".into());
            }
        }
    }

    errors
}

fn is_http_url(u: &str) -> bool {
    u.starts_with("http://") || u.starts_with("https://")
}

/// Push the shared "host is required" error when a host field is empty.
fn require_host(host: &str, errors: &mut Vec<String>) {
    if host.is_empty() {
        errors.push("host is required".into());
    }
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

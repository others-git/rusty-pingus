use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::db;
use crate::monitors::{MonitorConfig, MonitorStore};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub monitors: MonitorStore,
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
    pub uptime_24h: Option<f64>,
}

pub async fn list_monitors(State(state): State<AppState>) -> impl IntoResponse {
    let statuses = match db::get_current_status(&state.pool).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "DB error in list_monitors");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "db_error"}))).into_response();
        }
    };

    // Index the latest probe result by monitor name so we can join it against the
    // configured monitors.
    let mut status_by_name: HashMap<String, db::CurrentStatus> =
        statuses.into_iter().map(|s| (s.monitor_name.clone(), s)).collect();

    // The dashboard reflects the *configured* monitors (monitors.toml), joined with
    // their latest probe result. This excludes stale probe history for monitors that
    // are no longer configured, and shows configured-but-unprobed monitors as pending.
    let configured = state.monitors.list().await;
    let mut result = Vec::with_capacity(configured.len());
    for m in &configured {
        let name = m.name().to_string();
        if let Some(s) = status_by_name.remove(&name) {
            let uptime_24h = db::get_uptime(&state.pool, &name, 86_400).await.ok().flatten();
            result.push(MonitorStatus {
                name,
                protocol: s.protocol,
                endpoint: s.endpoint,
                status: s.status,
                last_checked_at: Some(s.checked_at),
                response_time_ms: s.response_time_ms,
                failure_reason: s.failure_reason,
                uptime_24h,
            });
        } else {
            result.push(MonitorStatus {
                name,
                protocol: m.protocol().to_string(),
                endpoint: m.endpoint(),
                status: "pending".to_string(),
                last_checked_at: None,
                response_time_ms: None,
                failure_reason: None,
                uptime_24h: None,
            });
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
    }

    errors
}

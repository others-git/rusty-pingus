use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::db;

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

pub async fn list_monitors(State(pool): State<SqlitePool>) -> impl IntoResponse {
    let statuses = match db::get_current_status(&pool).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "DB error in list_monitors");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "db_error"}))).into_response();
        }
    };

    let mut result = Vec::with_capacity(statuses.len());
    for s in statuses {
        let uptime_24h = db::get_uptime(&pool, &s.monitor_name, 86_400).await.ok().flatten();
        result.push(MonitorStatus {
            name: s.monitor_name,
            protocol: s.protocol,
            endpoint: s.endpoint,
            status: s.status,
            last_checked_at: Some(s.checked_at),
            response_time_ms: s.response_time_ms,
            failure_reason: s.failure_reason,
            uptime_24h,
        });
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
    State(pool): State<SqlitePool>,
    Path(name): Path<String>,
    Query(params): Query<HistoryParams>,
) -> impl IntoResponse {
    let exists = db::monitor_exists(&pool, &name).await.unwrap_or(false);
    if !exists {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "monitor_not_found", "name": name })),
        ).into_response();
    }

    let limit = params.limit.unwrap_or(100).min(1000);
    match db::get_history(&pool, &name, params.from, params.to, limit).await {
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
    State(pool): State<SqlitePool>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let exists = db::monitor_exists(&pool, &name).await.unwrap_or(false);
    if !exists {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "monitor_not_found", "name": name })),
        ).into_response();
    }

    let (u1h, u24h, u7d, u30d) = tokio::join!(
        db::get_uptime(&pool, &name, 3_600),
        db::get_uptime(&pool, &name, 86_400),
        db::get_uptime(&pool, &name, 604_800),
        db::get_uptime(&pool, &name, 2_592_000),
    );

    Json(UptimeResponse {
        uptime_1h: u1h.ok().flatten(),
        uptime_24h: u24h.ok().flatten(),
        uptime_7d: u7d.ok().flatten(),
        uptime_30d: u30d.ok().flatten(),
    }).into_response()
}

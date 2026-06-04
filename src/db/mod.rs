use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{sqlite::SqlitePoolOptions, FromRow, Row, SqlitePool};

use crate::probe::ProbeResult;

pub async fn init(path: &str) -> Result<SqlitePool> {
    let url = format!("sqlite://{}?mode=rwc", path);
    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect(&url)
        .await?;

    sqlx::query("PRAGMA journal_mode=WAL").execute(&pool).await?;
    sqlx::query("PRAGMA synchronous=NORMAL").execute(&pool).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

pub async fn insert_result(pool: &SqlitePool, result: &ProbeResult) -> Result<()> {
    sqlx::query(
        "INSERT INTO probe_results (monitor_name, protocol, endpoint, status, response_time_ms, failure_reason, checked_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&result.monitor_name)
    .bind(&result.protocol)
    .bind(&result.endpoint)
    .bind(&result.status)
    .bind(result.response_time_ms.map(|v| v as i64))
    .bind(&result.failure_reason)
    .bind(result.checked_at.to_rfc3339())
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Debug, Serialize, FromRow)]
pub struct CurrentStatus {
    pub monitor_name: String,
    pub protocol: String,
    pub endpoint: String,
    pub status: String,
    pub response_time_ms: Option<i64>,
    pub failure_reason: Option<String>,
    pub checked_at: String,
}

pub async fn get_current_status(pool: &SqlitePool) -> Result<Vec<CurrentStatus>> {
    let rows = sqlx::query_as::<_, CurrentStatus>(
        "SELECT monitor_name, protocol, endpoint, status, response_time_ms, failure_reason, checked_at
         FROM probe_results
         WHERE id IN (SELECT MAX(id) FROM probe_results GROUP BY monitor_name)
         ORDER BY monitor_name",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

#[derive(Debug, Serialize, FromRow)]
pub struct HistoryRow {
    pub id: i64,
    pub monitor_name: String,
    pub protocol: String,
    pub endpoint: String,
    pub status: String,
    pub response_time_ms: Option<i64>,
    pub failure_reason: Option<String>,
    pub checked_at: String,
}

pub async fn get_history(
    pool: &SqlitePool,
    monitor_name: &str,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    limit: i64,
) -> Result<Vec<HistoryRow>> {
    let from_str = from
        .map(|d| d.to_rfc3339())
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());
    let to_str = to
        .map(|d| d.to_rfc3339())
        .unwrap_or_else(|| "9999-12-31T23:59:59Z".to_string());

    let rows = sqlx::query_as::<_, HistoryRow>(
        "SELECT id, monitor_name, protocol, endpoint, status, response_time_ms, failure_reason, checked_at
         FROM probe_results
         WHERE monitor_name = ? AND checked_at >= ? AND checked_at <= ?
         ORDER BY checked_at DESC
         LIMIT ?",
    )
    .bind(monitor_name)
    .bind(&from_str)
    .bind(&to_str)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_uptime(
    pool: &SqlitePool,
    monitor_name: &str,
    window_secs: i64,
) -> Result<Option<f64>> {
    let from = Utc::now() - chrono::Duration::seconds(window_secs);
    let from_str = from.to_rfc3339();

    let row = sqlx::query(
        "SELECT COUNT(*) as total, SUM(CASE WHEN status = 'up' THEN 1 ELSE 0 END) as up_count
         FROM probe_results
         WHERE monitor_name = ? AND checked_at >= ?",
    )
    .bind(monitor_name)
    .bind(&from_str)
    .fetch_one(pool)
    .await?;

    let total: i64 = row.try_get("total")?;
    if total == 0 {
        return Ok(None);
    }
    let up: i64 = row.try_get("up_count").unwrap_or(0);
    Ok(Some((up as f64 / total as f64) * 100.0))
}

pub async fn prune_old_results(pool: &SqlitePool, retention_days: u64) -> Result<u64> {
    let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
    let cutoff_str = cutoff.to_rfc3339();
    let result = sqlx::query("DELETE FROM probe_results WHERE checked_at < ?")
        .bind(&cutoff_str)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// Delete all stored probe results for a monitor. Returns the number of rows removed.
pub async fn delete_results(pool: &SqlitePool, monitor_name: &str) -> Result<u64> {
    let result = sqlx::query("DELETE FROM probe_results WHERE monitor_name = ?")
        .bind(monitor_name)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn monitor_exists(pool: &SqlitePool, monitor_name: &str) -> Result<bool> {
    let row = sqlx::query("SELECT COUNT(*) as cnt FROM probe_results WHERE monitor_name = ?")
        .bind(monitor_name)
        .fetch_one(pool)
        .await?;
    let cnt: i64 = row.try_get("cnt")?;
    Ok(cnt > 0)
}

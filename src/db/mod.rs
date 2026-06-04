use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{FromRow, Row, SqlitePool};
use std::str::FromStr;

use crate::probe::ProbeResult;

pub async fn init(path: &str) -> Result<SqlitePool> {
    // Pragmas are applied per connection (via connect options) so every pooled
    // connection gets them — setting them once on the pool only affects one
    // connection. temp_store=MEMORY keeps GROUP BY/sort temp b-trees off disk
    // (important on slow filesystems), and a larger cache/mmap speeds the large
    // index scans used by the series/uptime aggregations.
    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", path))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .pragma("temp_store", "MEMORY")
        .pragma("cache_size", "-65536") // 64 MB page cache (negative = KiB)
        .pragma("mmap_size", "268435456"); // 256 MB memory-map

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(opts)
        .await?;

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

/// Latest probe result for a single monitor. O(1) index seek — used by the
/// dashboard so its cost scales with the number of monitors, not total rows.
pub async fn get_latest_status(pool: &SqlitePool, monitor_name: &str) -> Result<Option<CurrentStatus>> {
    let row = sqlx::query_as::<_, CurrentStatus>(
        "SELECT monitor_name, protocol, endpoint, status, response_time_ms, failure_reason, checked_at
         FROM probe_results
         WHERE monitor_name = ?
         ORDER BY checked_at DESC
         LIMIT 1",
    )
    .bind(monitor_name)
    .fetch_optional(pool)
    .await?;
    Ok(row)
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

#[derive(Debug, Serialize)]
pub struct SeriesBucket {
    pub ts: String,
    pub avg_ms: Option<f64>,
    pub min_ms: Option<i64>,
    pub max_ms: Option<i64>,
    pub count: i64,
    pub up_ratio: f64,
}

/// Aggregate a monitor's results over `[from, to]` into at most `buckets` time
/// buckets, each reporting avg/min/max response time, sample count, and up-ratio.
/// Bounds the output regardless of how many raw rows fall in the range.
pub async fn get_series(
    pool: &SqlitePool,
    monitor_name: &str,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    buckets: i64,
) -> Result<Vec<SeriesBucket>> {
    // Map each row into one of `n` slots spanning [from, to] so the output is
    // strictly bounded to `n` buckets regardless of volume or epoch alignment.
    let n = buckets.clamp(1, 5000);
    let from_epoch = from.timestamp();
    let span_secs = (to.timestamp() - from_epoch).max(1);
    let from_str = from.to_rfc3339();
    let to_str = to.to_rfc3339();

    let rows = sqlx::query(
        "SELECT MIN(? - 1, (CAST(strftime('%s', checked_at) AS INTEGER) - ?) * ? / ?) AS bucket,
                AVG(response_time_ms) AS avg_ms,
                MIN(response_time_ms) AS min_ms,
                MAX(response_time_ms) AS max_ms,
                COUNT(*) AS cnt,
                AVG(CASE WHEN status = 'up' THEN 1.0 ELSE 0.0 END) AS up_ratio
         FROM probe_results
         WHERE monitor_name = ? AND checked_at >= ? AND checked_at <= ?
         GROUP BY bucket
         ORDER BY bucket",
    )
    .bind(n)
    .bind(from_epoch)
    .bind(n)
    .bind(span_secs)
    .bind(monitor_name)
    .bind(&from_str)
    .bind(&to_str)
    .fetch_all(pool)
    .await?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let bucket: i64 = row.try_get("bucket")?;
        let ts_epoch = from_epoch + (bucket * span_secs) / n;
        let ts = DateTime::<Utc>::from_timestamp(ts_epoch, 0)
            .map(|d| d.to_rfc3339())
            .unwrap_or_default();
        out.push(SeriesBucket {
            ts,
            avg_ms: row.try_get("avg_ms").ok().flatten(),
            min_ms: row.try_get("min_ms").ok().flatten(),
            max_ms: row.try_get("max_ms").ok().flatten(),
            count: row.try_get("cnt").unwrap_or(0),
            up_ratio: row.try_get("up_ratio").unwrap_or(0.0),
        });
    }
    Ok(out)
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

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Row, SqlitePool};
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

    // Reclaim space left by a destructive migration (e.g. dropped columns). VACUUM
    // can't run inside a migration's transaction, so do it here, once, and only
    // when there's meaningful slack (cheap to check; no-op on a tight DB).
    let freelist: i64 = sqlx::query_scalar("PRAGMA freelist_count").fetch_one(&pool).await.unwrap_or(0);
    if freelist > 1000 {
        if let Err(e) = sqlx::query("VACUUM").execute(&pool).await {
            tracing::warn!(error = %e, "VACUUM after migration failed (space not reclaimed)");
        } else {
            tracing::info!(reclaimed_pages = freelist, "Compacted database (VACUUM)");
        }
    }

    Ok(pool)
}

/// Format a stored epoch-millisecond timestamp as the RFC3339 string the API has
/// always returned, so the on-disk integer encoding is invisible to clients.
fn epoch_ms_to_rfc3339(ms: i64) -> String {
    DateTime::<Utc>::from_timestamp_millis(ms).unwrap_or_default().to_rfc3339()
}

pub async fn insert_result(pool: &SqlitePool, result: &ProbeResult) -> Result<()> {
    // protocol/endpoint are constant per monitor and supplied from config at the
    // API layer, so they are no longer duplicated on every row. checked_at is an
    // integer epoch (ms).
    sqlx::query(
        "INSERT INTO probe_results (monitor_name, status, response_time_ms, failure_reason, detail, checked_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&result.monitor_name)
    .bind(&result.status)
    .bind(result.response_time_ms.map(|v| v as i64))
    .bind(&result.failure_reason)
    .bind(&result.detail)
    .bind(result.checked_at.timestamp_millis())
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct CurrentStatus {
    pub monitor_name: String,
    /// Supplied by the API layer from the monitor config (not stored per row).
    pub protocol: String,
    pub endpoint: String,
    pub status: String,
    pub response_time_ms: Option<i64>,
    pub failure_reason: Option<String>,
    pub detail: Option<String>,
    pub checked_at: String,
}

fn row_to_current(row: &sqlx::sqlite::SqliteRow) -> Result<CurrentStatus> {
    Ok(CurrentStatus {
        monitor_name: row.try_get("monitor_name")?,
        protocol: String::new(), // filled from config by the API layer
        endpoint: String::new(),
        status: row.try_get("status")?,
        response_time_ms: row.try_get("response_time_ms")?,
        failure_reason: row.try_get("failure_reason")?,
        detail: row.try_get("detail")?,
        checked_at: epoch_ms_to_rfc3339(row.try_get("checked_at")?),
    })
}

/// Latest probe result for a single monitor. O(1) index seek — used by the
/// dashboard so its cost scales with the number of monitors, not total rows.
pub async fn get_latest_status(pool: &SqlitePool, monitor_name: &str) -> Result<Option<CurrentStatus>> {
    let row = sqlx::query(
        "SELECT monitor_name, status, response_time_ms, failure_reason, detail, checked_at
         FROM probe_results
         WHERE monitor_name = ?
         ORDER BY checked_at DESC
         LIMIT 1",
    )
    .bind(monitor_name)
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(row_to_current).transpose()
}

pub async fn get_current_status(pool: &SqlitePool) -> Result<Vec<CurrentStatus>> {
    let rows = sqlx::query(
        "SELECT monitor_name, status, response_time_ms, failure_reason, detail, checked_at
         FROM probe_results
         WHERE id IN (SELECT MAX(id) FROM probe_results GROUP BY monitor_name)
         ORDER BY monitor_name",
    )
    .fetch_all(pool)
    .await?;
    rows.iter().map(row_to_current).collect()
}

#[derive(Debug, Serialize)]
pub struct HistoryRow {
    pub id: i64,
    pub monitor_name: String,
    /// Supplied by the API layer from the monitor config (not stored per row).
    pub protocol: String,
    pub endpoint: String,
    pub status: String,
    pub response_time_ms: Option<i64>,
    pub failure_reason: Option<String>,
    pub detail: Option<String>,
    pub checked_at: String,
}

pub async fn get_history(
    pool: &SqlitePool,
    monitor_name: &str,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    limit: i64,
) -> Result<Vec<HistoryRow>> {
    let from_ms = from.map(|d| d.timestamp_millis()).unwrap_or(i64::MIN);
    let to_ms = to.map(|d| d.timestamp_millis()).unwrap_or(i64::MAX);

    let rows = sqlx::query(
        "SELECT id, monitor_name, status, response_time_ms, failure_reason, detail, checked_at
         FROM probe_results
         WHERE monitor_name = ? AND checked_at >= ? AND checked_at <= ?
         ORDER BY checked_at DESC
         LIMIT ?",
    )
    .bind(monitor_name)
    .bind(from_ms)
    .bind(to_ms)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    rows.iter().map(|row| {
        Ok(HistoryRow {
            id: row.try_get("id")?,
            monitor_name: row.try_get("monitor_name")?,
            protocol: String::new(),
            endpoint: String::new(),
            status: row.try_get("status")?,
            response_time_ms: row.try_get("response_time_ms")?,
            failure_reason: row.try_get("failure_reason")?,
            detail: row.try_get("detail")?,
            checked_at: epoch_ms_to_rfc3339(row.try_get("checked_at")?),
        })
    }).collect()
}

pub async fn get_uptime(
    pool: &SqlitePool,
    monitor_name: &str,
    window_secs: i64,
) -> Result<Option<f64>> {
    // Long windows are served from the per-minute rollup (bounded cost); short
    // windows use raw rows (fast, and include the current minute).
    if window_secs > 86_400 {
        let from_epoch = (Utc::now() - chrono::Duration::seconds(window_secs)).timestamp();
        let row = sqlx::query(
            "SELECT SUM(count) AS total, SUM(up_count) AS up
             FROM probe_rollup_1m WHERE monitor_name = ? AND bucket_epoch >= ?",
        )
        .bind(monitor_name)
        .bind(from_epoch)
        .fetch_one(pool)
        .await?;
        let total: i64 = row.try_get::<Option<i64>, _>("total")?.unwrap_or(0);
        // If the rollup has data for this window, use it; otherwise (not yet
        // backfilled) fall through to the raw computation below.
        if total > 0 {
            let up: i64 = row.try_get::<Option<i64>, _>("up")?.unwrap_or(0);
            return Ok(Some((up as f64 / total as f64) * 100.0));
        }
    }

    let from_ms = (Utc::now() - chrono::Duration::seconds(window_secs)).timestamp_millis();

    let row = sqlx::query(
        "SELECT COUNT(*) as total, SUM(CASE WHEN status = 'up' THEN 1 ELSE 0 END) as up_count
         FROM probe_results
         WHERE monitor_name = ? AND checked_at >= ?",
    )
    .bind(monitor_name)
    .bind(from_ms)
    .fetch_one(pool)
    .await?;

    let total: i64 = row.try_get("total")?;
    if total == 0 {
        return Ok(None);
    }
    let up: i64 = row.try_get("up_count").unwrap_or(0);
    Ok(Some((up as f64 / total as f64) * 100.0))
}

/// 24h uptime % for every monitor in one query (for the dashboard list, so it
/// doesn't fan out a per-monitor uptime query). Keyed by monitor name; monitors
/// with no probes in the window are absent.
pub async fn get_uptime_24h_all(pool: &SqlitePool) -> Result<std::collections::HashMap<String, f64>> {
    let from_ms = (Utc::now() - chrono::Duration::hours(24)).timestamp_millis();
    let rows = sqlx::query(
        "SELECT monitor_name,
                COUNT(*) AS total,
                SUM(CASE WHEN status = 'up' THEN 1 ELSE 0 END) AS up_count
         FROM probe_results WHERE checked_at >= ? GROUP BY monitor_name",
    )
    .bind(from_ms)
    .fetch_all(pool)
    .await?;
    let mut map = std::collections::HashMap::with_capacity(rows.len());
    for row in rows {
        let total: i64 = row.try_get("total").unwrap_or(0);
        if total > 0 {
            let up: i64 = row.try_get("up_count").unwrap_or(0);
            let name: String = row.try_get("monitor_name")?;
            map.insert(name, (up as f64 / total as f64) * 100.0);
        }
    }
    Ok(map)
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
    let to_epoch = to.timestamp();
    let span_secs = (to_epoch - from_epoch).max(1);

    // When each output bucket spans >= 1 minute, serve from the per-minute rollup
    // (cost scales with minutes, not raw rows). If the rollup has no data for the
    // range yet (e.g. startup backfill not finished), fall back to the raw path so
    // results are correct (just not yet fast). Finer ranges always use raw.
    if span_secs / n >= 60 {
        let rollup = get_series_rollup(pool, monitor_name, from_epoch, to_epoch, n, span_secs).await?;
        if !rollup.is_empty() {
            return Ok(rollup);
        }
    }

    // checked_at is epoch ms; bucket on the millisecond span (no per-row parsing).
    let from_ms = from.timestamp_millis();
    let to_ms = to.timestamp_millis();
    let span_ms = (to_ms - from_ms).max(1);

    let rows = sqlx::query(
        "SELECT MIN(? - 1, (checked_at - ?) * ? / ?) AS bucket,
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
    .bind(from_ms)
    .bind(n)
    .bind(span_ms)
    .bind(monitor_name)
    .bind(from_ms)
    .bind(to_ms)
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

/// Rollup-backed series: aggregate per-minute rollups over `[from,to]` into `n`
/// bounded buckets. avg = SUM(sum_ms)/SUM(up_count) (over successful probes).
async fn get_series_rollup(
    pool: &SqlitePool,
    monitor_name: &str,
    from_epoch: i64,
    to_epoch: i64,
    n: i64,
    span_secs: i64,
) -> Result<Vec<SeriesBucket>> {
    let rows = sqlx::query(
        "SELECT MIN(? - 1, (bucket_epoch - ?) * ? / ?) AS bucket,
                SUM(sum_ms) AS sum_ms,
                SUM(up_count) AS up_count,
                MIN(min_ms) AS min_ms,
                MAX(max_ms) AS max_ms,
                SUM(count) AS cnt
         FROM probe_rollup_1m
         WHERE monitor_name = ? AND bucket_epoch >= ? AND bucket_epoch <= ?
         GROUP BY bucket
         ORDER BY bucket",
    )
    .bind(n)
    .bind(from_epoch)
    .bind(n)
    .bind(span_secs)
    .bind(monitor_name)
    .bind(from_epoch)
    .bind(to_epoch)
    .fetch_all(pool)
    .await?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let bucket: i64 = row.try_get("bucket")?;
        let cnt: i64 = row.try_get("cnt").unwrap_or(0);
        let up_count: i64 = row.try_get("up_count").unwrap_or(0);
        let sum_ms: i64 = row.try_get("sum_ms").unwrap_or(0);
        let ts_epoch = from_epoch + (bucket * span_secs) / n;
        let ts = DateTime::<Utc>::from_timestamp(ts_epoch, 0)
            .map(|d| d.to_rfc3339())
            .unwrap_or_default();
        out.push(SeriesBucket {
            ts,
            avg_ms: if up_count > 0 { Some(sum_ms as f64 / up_count as f64) } else { None },
            min_ms: row.try_get("min_ms").ok().flatten(),
            max_ms: row.try_get("max_ms").ok().flatten(),
            count: cnt,
            up_ratio: if cnt > 0 { up_count as f64 / cnt as f64 } else { 0.0 },
        });
    }
    Ok(out)
}

// ── Rollup maintenance ─────────────────────────────────────────────────────────

/// Latest rolled-up minute (epoch), or None if the rollup is empty.
pub async fn rollup_watermark(pool: &SqlitePool) -> Result<Option<i64>> {
    let row = sqlx::query("SELECT MAX(bucket_epoch) AS m FROM probe_rollup_1m")
        .fetch_one(pool)
        .await?;
    Ok(row.try_get::<Option<i64>, _>("m")?)
}

/// Earliest raw probe minute (epoch floored to the minute), or None if no data.
pub async fn earliest_raw_minute(pool: &SqlitePool) -> Result<Option<i64>> {
    // checked_at is epoch ms; floor to the minute in epoch seconds.
    let row = sqlx::query(
        "SELECT (MIN(checked_at) / 60000) * 60 AS m FROM probe_results",
    )
    .fetch_one(pool)
    .await?;
    Ok(row.try_get::<Option<i64>, _>("m")?)
}

/// Aggregate raw results in `[from_epoch, to_epoch)` into per-minute rollup rows
/// (INSERT OR REPLACE, idempotent). Returns rows affected.
pub async fn roll_up_range(pool: &SqlitePool, from_epoch: i64, to_epoch: i64) -> Result<u64> {
    let result = sqlx::query(
        "INSERT OR REPLACE INTO probe_rollup_1m
            (monitor_name, bucket_epoch, count, up_count, sum_ms, min_ms, max_ms)
         SELECT monitor_name,
                (checked_at / 60000) * 60 AS b,
                COUNT(*),
                SUM(CASE WHEN status = 'up' THEN 1 ELSE 0 END),
                SUM(CASE WHEN status = 'up' THEN response_time_ms ELSE 0 END),
                MIN(response_time_ms),
                MAX(response_time_ms)
         FROM probe_results
         WHERE checked_at >= ? AND checked_at < ?
         GROUP BY monitor_name, b",
    )
    .bind(from_epoch * 1000)
    .bind(to_epoch * 1000)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn prune_old_rollups(pool: &SqlitePool, retention_days: u64) -> Result<u64> {
    let cutoff = (Utc::now() - chrono::Duration::days(retention_days as i64)).timestamp();
    let result = sqlx::query("DELETE FROM probe_rollup_1m WHERE bucket_epoch < ?")
        .bind(cutoff)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn prune_old_results(pool: &SqlitePool, retention_days: u64) -> Result<u64> {
    let cutoff_ms = (Utc::now() - chrono::Duration::days(retention_days as i64)).timestamp_millis();
    let result = sqlx::query("DELETE FROM probe_results WHERE checked_at < ?")
        .bind(cutoff_ms)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// Delete probe_results for one monitor older than `cutoff_ms` (epoch ms).
pub async fn prune_monitor_results(pool: &SqlitePool, monitor_name: &str, cutoff_ms: i64) -> Result<u64> {
    let result = sqlx::query("DELETE FROM probe_results WHERE monitor_name = ? AND checked_at < ?")
        .bind(monitor_name)
        .bind(cutoff_ms)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// Delete rollup rows for one monitor older than `cutoff_epoch` (epoch seconds).
pub async fn prune_monitor_rollups(pool: &SqlitePool, monitor_name: &str, cutoff_epoch: i64) -> Result<u64> {
    let result = sqlx::query("DELETE FROM probe_rollup_1m WHERE monitor_name = ? AND bucket_epoch < ?")
        .bind(monitor_name)
        .bind(cutoff_epoch)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// The earliest and latest probe times (epoch ms) for a monitor. Used to
/// supply brush bounds on the detail page. Returns None when there is no data.
pub async fn get_probe_extent(pool: &SqlitePool, monitor_name: &str) -> Result<Option<(i64, i64)>> {
    let row = sqlx::query(
        "SELECT MIN(checked_at) AS lo, MAX(checked_at) AS hi FROM probe_results WHERE monitor_name = ?",
    )
    .bind(monitor_name)
    .fetch_one(pool)
    .await?;
    let lo: Option<i64> = row.try_get("lo")?;
    let hi: Option<i64> = row.try_get("hi")?;
    Ok(match (lo, hi) {
        (Some(lo), Some(hi)) => Some((lo, hi)),
        _ => None,
    })
}

/// A collapsed state run — a contiguous span where the probe's leading detail
/// token (the IP for public-IP monitors; the fault class for border) was the
/// same value. Null state means the monitor was down/no detail for that span.
#[derive(Debug, Serialize)]
pub struct StateSegment {
    pub state: Option<String>,
    pub start_ms: i64,
    pub end_ms: i64,
}

/// Return collapsed state segments for a monitor over `[from_ms, to_ms]`, scanning
/// ascending and merging consecutive rows that share the same leading detail token.
/// The result is bounded by the number of state *changes*, not the number of probes,
/// so a fast monitor's full window is covered regardless of probe frequency.
pub async fn get_state_segments(
    pool: &SqlitePool,
    monitor_name: &str,
    from_ms: i64,
    to_ms: i64,
) -> Result<Vec<StateSegment>> {
    let rows = sqlx::query(
        "SELECT detail, checked_at FROM probe_results
         WHERE monitor_name = ? AND checked_at >= ? AND checked_at <= ?
         ORDER BY checked_at ASC",
    )
    .bind(monitor_name)
    .bind(from_ms)
    .bind(to_ms)
    .fetch_all(pool)
    .await?;

    let mut segs: Vec<StateSegment> = Vec::new();
    for row in &rows {
        let detail: Option<String> = row.try_get("detail")?;
        let t: i64 = row.try_get("checked_at")?;
        // Leading token of detail: the IP address for publicip, fault class for border.
        let state = detail.as_deref()
            .and_then(|d| d.trim().split_whitespace().next())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        match segs.last_mut() {
            Some(last) if last.state == state => {
                last.end_ms = t;
            }
            _ => {
                if let Some(last) = segs.last_mut() {
                    last.end_ms = t;
                }
                segs.push(StateSegment { state, start_ms: t, end_ms: t });
            }
        }
    }
    if let Some(last) = segs.last_mut() {
        last.end_ms = to_ms;
    }
    Ok(segs)
}

/// Delete all stored probe results (and rollups) for a monitor. Returns the
/// number of raw rows removed.
pub async fn delete_results(pool: &SqlitePool, monitor_name: &str) -> Result<u64> {
    let result = sqlx::query("DELETE FROM probe_results WHERE monitor_name = ?")
        .bind(monitor_name)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM probe_rollup_1m WHERE monitor_name = ?")
        .bind(monitor_name)
        .execute(pool)
        .await?;
    // Also clear any traceroute data for this monitor (best-effort; ignore if none).
    let _ = delete_traceroute(pool, monitor_name).await;
    Ok(result.rows_affected())
}

// ── Traceroute storage ─────────────────────────────────────────────────────────

/// One hop's per-run statistics, as produced by the traceroute probe and handed
/// to storage. A non-responding hop has `addr = None` and null RTTs; `loss` is the
/// number of the run's queries at this hop that got no reply.
#[derive(Debug, Clone)]
pub struct TraceHopInput {
    pub hop_no: i64,
    pub addr: Option<String>,
    pub min_us: Option<i64>,
    pub avg_us: Option<i64>,
    pub max_us: Option<i64>,
    pub loss: i64,
}

/// A single traceroute run: whether the destination replied, and its hops.
#[derive(Debug, Clone)]
pub struct TraceRunInput {
    pub reached: bool,
    pub hops: Vec<TraceHopInput>,
}

/// Persist one traceroute run: insert the run, intern each hop address (so a
/// stable route stores each address once), and write the hop rows — all in one
/// transaction. Addresses are cached within the run so a repeated hop address is
/// interned with a single lookup.
pub async fn insert_traceroute(
    pool: &SqlitePool,
    monitor_name: &str,
    checked_at: DateTime<Utc>,
    run: &TraceRunInput,
) -> Result<()> {
    let mut tx = pool.begin().await?;

    let run_id: i64 = sqlx::query(
        "INSERT INTO traceroute_runs (monitor_name, checked_at, reached, hop_count)
         VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind(monitor_name)
    .bind(checked_at.timestamp_millis())
    .bind(if run.reached { 1 } else { 0 })
    .bind(run.hops.len() as i64)
    .fetch_one(&mut *tx)
    .await?
    .try_get("id")?;

    // Intern addresses once per distinct value within this run.
    let mut addr_cache: std::collections::HashMap<&str, i64> = std::collections::HashMap::new();
    for hop in &run.hops {
        let addr_id: Option<i64> = match &hop.addr {
            Some(addr) => {
                if let Some(&id) = addr_cache.get(addr.as_str()) {
                    Some(id)
                } else {
                    sqlx::query("INSERT INTO traceroute_addrs (addr) VALUES (?) ON CONFLICT(addr) DO NOTHING")
                        .bind(addr)
                        .execute(&mut *tx)
                        .await?;
                    let id: i64 = sqlx::query("SELECT id FROM traceroute_addrs WHERE addr = ?")
                        .bind(addr)
                        .fetch_one(&mut *tx)
                        .await?
                        .try_get("id")?;
                    addr_cache.insert(addr.as_str(), id);
                    Some(id)
                }
            }
            None => None,
        };

        sqlx::query(
            "INSERT INTO traceroute_hops (run_id, hop_no, addr_id, min_us, avg_us, max_us, loss)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(run_id)
        .bind(hop.hop_no)
        .bind(addr_id)
        .bind(hop.min_us)
        .bind(hop.avg_us)
        .bind(hop.max_us)
        .bind(hop.loss)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Per-hop aggregate over a time range, for the traceroute detail table.
#[derive(Debug, Serialize)]
pub struct TraceHopAgg {
    pub hop_no: i64,
    /// Most recent address seen at this hop in range, or null if it never responded.
    pub addr: Option<String>,
    pub reachable: bool,
    pub samples: i64,
    pub loss: i64,
    pub min_ms: Option<f64>,
    pub avg_ms: Option<f64>,
    pub max_ms: Option<f64>,
}

/// Aggregate a monitor's traceroute hops over `[from, to]`: per hop position,
/// reachability and min/avg/max RTT across the runs in the range, with the
/// address taken from the most recent run in range.
pub async fn get_traceroute_hops(
    pool: &SqlitePool,
    monitor_name: &str,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<TraceHopAgg>> {
    let from_ms = from.timestamp_millis();
    let to_ms = to.timestamp_millis();

    // Aggregates per hop position across all runs in range (RTTs in microseconds).
    let rows = sqlx::query(
        // CAST MIN/MAX to REAL: over an INTEGER column they keep integer affinity,
        // which fails to decode as f64; AVG already yields REAL.
        "SELECT h.hop_no AS hop_no,
                CAST(MIN(h.min_us) AS REAL) AS min_us,
                AVG(h.avg_us) AS avg_us,
                CAST(MAX(h.max_us) AS REAL) AS max_us,
                SUM(h.loss) AS loss,
                COUNT(*) AS samples,
                SUM(CASE WHEN h.addr_id IS NOT NULL THEN 1 ELSE 0 END) AS responded
         FROM traceroute_hops h
         JOIN traceroute_runs r ON h.run_id = r.id
         WHERE r.monitor_name = ? AND r.checked_at >= ? AND r.checked_at <= ?
         GROUP BY h.hop_no
         ORDER BY h.hop_no",
    )
    .bind(monitor_name)
    .bind(from_ms)
    .bind(to_ms)
    .fetch_all(pool)
    .await?;

    // Address per hop from the most recent run in range (the current path).
    let latest_run: Option<i64> = sqlx::query(
        "SELECT MAX(id) AS id FROM traceroute_runs
         WHERE monitor_name = ? AND checked_at >= ? AND checked_at <= ?",
    )
    .bind(monitor_name)
    .bind(from_ms)
    .bind(to_ms)
    .fetch_one(pool)
    .await?
    .try_get::<Option<i64>, _>("id")?;

    let mut addr_by_hop: std::collections::HashMap<i64, String> = std::collections::HashMap::new();
    if let Some(run_id) = latest_run {
        let arows = sqlx::query(
            "SELECT h.hop_no AS hop_no, a.addr AS addr
             FROM traceroute_hops h
             JOIN traceroute_addrs a ON h.addr_id = a.id
             WHERE h.run_id = ?",
        )
        .bind(run_id)
        .fetch_all(pool)
        .await?;
        for row in arows {
            let hop_no: i64 = row.try_get("hop_no")?;
            let addr: String = row.try_get("addr")?;
            addr_by_hop.insert(hop_no, addr);
        }
    }

    let us_to_ms = |v: Option<f64>| v.map(|u| u / 1000.0);
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let hop_no: i64 = row.try_get("hop_no")?;
        let responded: i64 = row.try_get("responded").unwrap_or(0);
        out.push(TraceHopAgg {
            hop_no,
            addr: addr_by_hop.get(&hop_no).cloned(),
            reachable: responded > 0,
            samples: row.try_get("samples").unwrap_or(0),
            loss: row.try_get("loss").unwrap_or(0),
            min_ms: us_to_ms(row.try_get::<Option<f64>, _>("min_us").ok().flatten()),
            avg_ms: us_to_ms(row.try_get::<Option<f64>, _>("avg_us").ok().flatten()),
            max_ms: us_to_ms(row.try_get::<Option<f64>, _>("max_us").ok().flatten()),
        });
    }
    Ok(out)
}

/// The retained time extent of a monitor's traceroute data (earliest/latest run
/// timestamps), so the UI brush knows its bounds. None when there is no data.
pub async fn get_traceroute_extent(
    pool: &SqlitePool,
    monitor_name: &str,
) -> Result<Option<(String, String)>> {
    let row = sqlx::query(
        "SELECT MIN(checked_at) AS lo, MAX(checked_at) AS hi
         FROM traceroute_runs WHERE monitor_name = ?",
    )
    .bind(monitor_name)
    .fetch_one(pool)
    .await?;
    let lo: Option<i64> = row.try_get("lo")?;
    let hi: Option<i64> = row.try_get("hi")?;
    Ok(match (lo, hi) {
        (Some(lo), Some(hi)) => Some((epoch_ms_to_rfc3339(lo), epoch_ms_to_rfc3339(hi))),
        _ => None,
    })
}

/// Prune a monitor's traceroute runs (and their hops) older than `retention_ms`,
/// then release interned addresses no longer referenced by any hop. Returns the
/// number of runs deleted. Foreign-key cascade is not relied upon (PRAGMA
/// foreign_keys is off), so hops are deleted explicitly.
pub async fn prune_traceroute(
    pool: &SqlitePool,
    monitor_name: &str,
    retention_ms: u64,
) -> Result<u64> {
    let cutoff_ms = (Utc::now() - chrono::Duration::milliseconds(retention_ms as i64)).timestamp_millis();
    let mut tx = pool.begin().await?;

    sqlx::query(
        "DELETE FROM traceroute_hops WHERE run_id IN
            (SELECT id FROM traceroute_runs WHERE monitor_name = ? AND checked_at < ?)",
    )
    .bind(monitor_name)
    .bind(cutoff_ms)
    .execute(&mut *tx)
    .await?;

    let deleted = sqlx::query(
        "DELETE FROM traceroute_runs WHERE monitor_name = ? AND checked_at < ?",
    )
    .bind(monitor_name)
    .bind(cutoff_ms)
    .execute(&mut *tx)
    .await?
    .rows_affected();

    sqlx::query(
        "DELETE FROM traceroute_addrs WHERE id NOT IN
            (SELECT addr_id FROM traceroute_hops WHERE addr_id IS NOT NULL)",
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(deleted)
}

/// Delete all of a monitor's traceroute data (runs, hops) and GC orphan addresses.
/// Used when a monitor is removed.
pub async fn delete_traceroute(pool: &SqlitePool, monitor_name: &str) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "DELETE FROM traceroute_hops WHERE run_id IN
            (SELECT id FROM traceroute_runs WHERE monitor_name = ?)",
    )
    .bind(monitor_name)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM traceroute_runs WHERE monitor_name = ?")
        .bind(monitor_name)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "DELETE FROM traceroute_addrs WHERE id NOT IN
            (SELECT addr_id FROM traceroute_hops WHERE addr_id IS NOT NULL)",
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn monitor_exists(pool: &SqlitePool, monitor_name: &str) -> Result<bool> {
    let row = sqlx::query("SELECT COUNT(*) as cnt FROM probe_results WHERE monitor_name = ?")
        .bind(monitor_name)
        .fetch_one(pool)
        .await?;
    let cnt: i64 = row.try_get("cnt")?;
    Ok(cnt > 0)
}

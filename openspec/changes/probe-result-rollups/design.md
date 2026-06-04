## Context

`get_series` and `get_uptime` aggregate raw `probe_results` over a range. With the covering index `(monitor_name, checked_at, status, response_time_ms)` they're index-only, but still O(rows): ~5.2M for a 30-day, 500ms monitor. Recent/narrow ranges are small and fast; only wide ranges (7d/30d series, 7d/30d uptime) are slow, and they grow with retention. `checked_at` is ISO-8601 text. The scheduler already runs background loops (per-monitor probes, daily retention) and the binary spawns them from `main`.

## Goals / Non-Goals

**Goals:**
- Sub-second wide-window series and uptime on multi-million-row datasets.
- Exact-enough aggregates: count, up-ratio, min/max exact; average over successful probes.
- Keep the existing API shapes and frontend unchanged.
- Self-healing maintenance: backfill on startup, incremental thereafter.

**Non-Goals:**
- Hour/day rollup tiers (single per-minute tier is enough here).
- Changing raw retention or the raw write path.
- Rollup of sub-minute detail (zoomed-in views use raw).

## Decisions

### 1. Schema: `probe_rollup_1m`
```sql
CREATE TABLE probe_rollup_1m (
  monitor_name TEXT    NOT NULL,
  bucket_epoch INTEGER NOT NULL,   -- unix seconds at minute start (multiple of 60)
  count        INTEGER NOT NULL,   -- total probes in the minute
  up_count     INTEGER NOT NULL,   -- 'up' probes
  sum_ms       INTEGER NOT NULL,   -- sum of response_time_ms over 'up' probes
  min_ms       INTEGER,            -- over 'up' probes
  max_ms       INTEGER,
  PRIMARY KEY (monitor_name, bucket_epoch)
) WITHOUT ROWID;
```
Storing `sum_ms` + `up_count` (not a pre-divided average) makes re-bucketing exact: for a chart bucket spanning several minutes, `avg = SUM(sum_ms)/SUM(up_count)`, `up_ratio = SUM(up_count)/SUM(count)`, `min = MIN(min_ms)`, `max = MAX(max_ms)`, `count = SUM(count)`. This matches raw semantics where `AVG/MIN/MAX(response_time_ms)` ignore `down` rows (NULL response time). `WITHOUT ROWID` + the composite PK keeps the table compact and range-scannable by `(monitor_name, bucket_epoch)`.

### 2. Maintenance via a background task with a watermark
A loop ticks every ~30s. The watermark is `MAX(bucket_epoch)` in the rollup. Each tick aggregates complete minutes in `[watermark, now_minute)` (re-rolling the watermark minute with `INSERT OR REPLACE` in case it was previously partial), grouping raw rows by `(monitor_name, minute)`:
```sql
INSERT OR REPLACE INTO probe_rollup_1m
SELECT monitor_name, (CAST(strftime('%s',checked_at) AS INTEGER)/60)*60 AS b,
       COUNT(*), SUM(status='up'),
       SUM(CASE WHEN status='up' THEN response_time_ms ELSE 0 END),
       MIN(response_time_ms), MAX(response_time_ms)
FROM probe_results
WHERE checked_at >= :from AND checked_at < :to
GROUP BY monitor_name, b;
```
The current (incomplete) minute is excluded so it isn't rolled prematurely. **Startup backfill**: if the rollup is empty, the watermark is the earliest raw minute, so the first run aggregates all history (one-time ~seconds on 5.2M rows). To bound transaction size and memory, backfill proceeds in chunks (e.g. one day of minutes per statement). Alternative — incremental-on-write — rejected (couples to the hot insert path; every probe pays).

### 3. Query routing
- **Series**: `bucket_secs = span / buckets`. If `bucket_secs >= 60`, read the rollup (re-bucket minute rows into ≤N buckets); else read raw. 300 buckets over ≥5h → rollup; tighter zoom → raw. This dovetails with the dynamic-resolution frontend (wide = rollup/fast, zoomed-in = raw/already-fast).
- **Uptime**: windows `> 24h` (7d, 30d) read the rollup (`SUM(up_count)/SUM(count)`); `≤ 24h` (1h, 24h) read raw (fast, and includes the live current minute).
Routing lives in `db` so handlers are unchanged.

### 4. Accuracy and lag
Rollups lag by up to one tick + the current minute. Wide views (24h–30d) are unaffected in practice (one missing minute is invisible at that scale), and short/recent views use raw. Down probes contribute to `count` and `up_ratio` but not to `sum_ms/min/max`, identical to the raw `AVG/MIN/MAX(response_time_ms)` NULL-skipping behavior.

### 5. Retention
The retention loop also prunes `probe_rollup_1m` rows older than the retention cutoff, so rollups don't outlive raw data. Deleting a monitor (existing `delete_results`) also deletes its rollup rows.

## Risks / Trade-offs

- **Initial backfill cost on the existing 5.2M-row DB** → one-time, chunked, runs in the background after startup; queries fall back to raw until the rollup is populated (correct, just not yet fast). Mitigated by chunking so it doesn't hold a long write lock.
- **Double storage** → minute rollups are tiny (~43k rows/monitor/30d) vs millions of raw rows; negligible.
- **Watermark correctness across restarts/gaps** → recomputed from `MAX(bucket_epoch)` each startup; any gap (downtime) is backfilled because the range starts at the watermark.
- **A monitor deleted then re-added with the same name** → `delete_results` clears its rollup too, so stale aggregates don't resurface.
- **Series/uptime momentarily fall back to raw** while the rollup is still backfilling → slower but correct during that window; acceptable and self-resolving.

## Migration Plan

1. Ship migration `003` (rollup table) — additive, no raw-data change.
2. On first start, the maintenance task backfills the rollup from existing raw data (background, chunked); wide queries get fast once it completes.
3. Rollback: revert the binary; the rollup table is ignored by older code and can be dropped. No raw-data impact.

## Open Questions

_None — schema, maintenance, routing thresholds, and retention interplay are decided above._

## Why

Wide chart windows and long uptime windows aggregate every raw `probe_results` row in range on each request. At low poll intervals this is millions of rows: a 30-day series over a 500ms monitor scans ~5.2M rows (~3s in a release build, far worse in debug), and `uptime_30d` scans the same. Indexing made these index-only but they remain O(rows), so the fully-zoomed-out views and long uptime windows are not snappy and the cost grows with retention. Pre-aggregating into per-minute rollups makes wide queries O(minutes) — 30 days becomes ~43k rows instead of ~5.2M.

## What Changes

- **Per-minute rollup table** (`probe_rollup_1m`): for each monitor and each minute, store `count`, `up_count`, `sum_ms` (sum of `up` response times), `min_ms`, `max_ms`. Re-aggregating minute rows into chart buckets is exact for count/up-ratio/min/max and for average over successful probes.
- **Background rollup maintenance**: a periodic task aggregates newly-completed minutes from `probe_results` into the rollup. On startup it backfills any gap (covers existing data and downtime); for the current 5.2M-row DB this is a one-time backfill that runs in the background, not blocking startup.
- **Query routing** (same API contracts, just faster):
  - Aggregated series: when each output bucket spans ≥1 minute (wide/zoomed-out ranges), read the rollup; for finer ranges read raw `probe_results` (already fast on small ranges, and accurate to the live tail).
  - Uptime: long windows (7d/30d) read the rollup; short windows (1h/24h) read raw.
- **Rollups respect retention**: rollup rows older than the retention period are pruned alongside raw results.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `result-storage`: Add a per-minute rollup table and a background maintenance process that keeps it current (incremental + startup backfill); the aggregated series query and the uptime calculation use rollups for wide ranges/long windows (bounded, sub-second on large datasets) and raw results for fine ranges/short windows; rollups are pruned by the retention policy.

## Impact

- `migrations/003_rollups.sql` — new `probe_rollup_1m` table + index.
- `src/db/mod.rs` — rollup maintenance queries (incremental upsert, startup backfill, watermark), rollup-backed `get_series`/`get_uptime` paths, rollup pruning.
- `src/scheduler/mod.rs` — background rollup-maintenance loop (and run pruning over rollups in the retention loop).
- `src/main.rs` — spawn the rollup-maintenance task.
- `src/api/mod.rs` — series/uptime handlers route to rollup vs raw (or routing lives in the db layer).
- `README.md` — note minute-rollups power fast wide-window charts and uptime.
- No change to the JSON API shapes or the frontend.

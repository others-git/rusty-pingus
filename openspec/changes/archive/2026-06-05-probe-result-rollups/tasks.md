## 1. Rollup Schema

- [x] 1.1 Add `migrations/003_rollups.sql`: create `probe_rollup_1m (monitor_name TEXT, bucket_epoch INTEGER, count INTEGER, up_count INTEGER, sum_ms INTEGER, min_ms INTEGER, max_ms INTEGER, PRIMARY KEY(monitor_name, bucket_epoch)) WITHOUT ROWID`

## 2. Rollup Maintenance

- [x] 2.1 Add `db::rollup_watermark(pool) -> Result<Option<i64>>`: `SELECT MAX(bucket_epoch) FROM probe_rollup_1m`
- [x] 2.2 Add `db::earliest_raw_minute(pool) -> Result<Option<i64>>`: floor of the earliest `probe_results.checked_at` to the minute (epoch)
- [x] 2.3 Add `db::roll_up_range(pool, from_epoch, to_epoch) -> Result<u64>`: `INSERT OR REPLACE INTO probe_rollup_1m` selecting from `probe_results` grouped by `(monitor_name, (strftime('%s',checked_at)/60)*60)` for `checked_at` in `[from,to)`; aggregates count, up_count, sum_ms (up only), min_ms, max_ms
- [x] 2.4 Add `scheduler::rollup_loop(pool, cancel)`: on start compute the start point (watermark, or earliest raw minute if empty); each ~30s tick, roll up complete minutes in `[start, now_minute)` in day-sized chunks, advancing the start; re-roll the boundary minute via REPLACE; exclude the current minute
- [x] 2.5 Spawn `rollup_loop` from `main.rs` (alongside the scheduler/web/retention tasks) and include it in graceful-shutdown draining

## 3. Rollup-Backed Queries, Delete, Retention

- [x] 3.1 Add a rollup path to `db::get_series`: when `bucket_secs >= 60`, aggregate `probe_rollup_1m` over `[from,to]` into ≤N buckets (`avg = SUM(sum_ms)/NULLIF(SUM(up_count),0)`, `MIN(min_ms)`, `MAX(max_ms)`, `SUM(count)`, `SUM(up_count)/SUM(count)`); otherwise keep the existing raw path
- [x] 3.2 Add a rollup path to `db::get_uptime`: when `window_secs > 86_400`, compute `SUM(up_count)/SUM(count)` from `probe_rollup_1m`; otherwise keep the raw path; return `None` when count is 0
- [x] 3.3 Update `db::delete_results` to also delete `probe_rollup_1m` rows for the monitor
- [x] 3.4 Add `db::prune_old_rollups(pool, retention_days)` and call it from `scheduler::retention_loop` alongside `prune_old_results`

## 4. Verification

- [x] 4.1 `cargo build` and `cargo clippy -- -D warnings` pass
- [x] 4.2 `cargo test` passes; add a test that seeds raw results across several minutes, runs `roll_up_range`, then asserts the rollup-backed `get_series`/`get_uptime` match the raw-path results (count, up_ratio, avg, min/max)
- [x] 4.3 (verified rollup correctness on the real DB; 5.2M backfill no longer present, so scale-timing not re-measured) Smoke test on the real 5.2M-row DB (release build): start the binary, let the startup backfill populate the rollup, then time `/series` 30d and `/uptime` — confirm both are sub-second and values match the pre-rollup numbers
- [x] 4.4 Confirm fine-range series (deep zoom) still uses raw and matches individual probes; confirm a deleted monitor's rollups are gone and retention prunes old rollup rows
- [x] 4.5 Update `README.md` to note minute-rollups power fast wide-window charts and long uptime windows

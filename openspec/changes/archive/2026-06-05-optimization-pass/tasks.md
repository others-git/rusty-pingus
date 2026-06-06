## 1. Database migration (compact storage)

- [x] 1.1 Add `migrations/006_compact_storage.sql`: re-encode `probe_results.checked_at` TEXT→INTEGER epoch ms (`CAST((julianday(checked_at) - 2440587.5) * 86400000 AS INTEGER)`), drop `protocol` and `endpoint`, and recreate the covering index over the integer `checked_at`.
- [x] 1.2 In the same migration, re-encode `traceroute_runs.checked_at` the same way, and drop `traceroute_hops.rtt_us`.
- [x] 1.3 End the migration with `VACUUM` to reclaim freed space. Guard each table transform so re-running is safe.

## 2. Data layer: timestamp encoding (db/mod.rs)

- [x] 2.1 Change row structs (`CurrentStatus`, `HistoryRow`) to read `checked_at` as `i64` epoch ms and serialize as RFC3339 via a small `epoch_ms_to_rfc3339` helper.
- [x] 2.2 Update `insert_result` and `insert_traceroute` to bind `checked_at` as epoch ms (`checked_at.timestamp_millis()`).
- [x] 2.3 Update range filters and bucket math to integers: `get_history` (compare ms), `get_uptime` (cutoff ms; drop `strftime`), `get_series`/`get_series_rollup` (bucket from ms), `roll_up_range` / `earliest_raw_minute` (derive minute epoch from ms), `prune_old_results` and traceroute prune/extent/range (ms cutoffs and ISO out).
- [x] 2.4 Remove the `rtt_us` bind from `insert_traceroute`.

## 3. Data layer: drop per-row protocol/endpoint + batched listing

- [x] 3.1 Remove `protocol`/`endpoint` from the `INSERT` in `insert_result` and from the row structs' DB reads; the structs keep the fields but they are populated by the API layer (below), not the table.
- [x] 3.2 Rework `list_monitors` (api/mod.rs) to use `get_current_status` (one query) plus one batched 24h-uptime query, assembling per monitor in memory and filling `protocol`/`endpoint` from `MonitorStore`.
- [x] 3.3 In `monitor_history` and current-status responses, inject the monitor's `protocol`/`endpoint` from `MonitorStore` (neutral fallback when the monitor no longer exists), so the detail page still receives `protocol`.

## 4. Rust DRY (monitors/mod.rs, api/mod.rs)

- [x] 4.1 Add a `resolve_timing(...) -> (interval_ms, timeout_ms)` helper and use it in all five `From<Raw*MonitorConfig>` impls (preserving the traceroute 500 ms floor applied afterward).
- [x] 4.2 Add `apply_timing_defaults(&mut interval_ms, &mut timeout_ms, &Defaults)` and collapse each `apply_defaults` arm to a single call (re-applying the traceroute floor).
- [x] 4.3 Tidy `validate_monitor` where per-variant blocks repeat (e.g. shared host validation), without changing the errors returned.

## 5. Frontend DRY (assets/)

- [x] 5.1 Add `assets/common.js` exporting `formatRelative` and a `subscribeStatus({ name?, onUpdate, onError })` helper wrapping EventSource subscribe + poll fallback.
- [x] 5.2 Replace the duplicated `formatRelative` in `app.js` and `monitor.js` with the shared one; route their SSE/poll setup through `subscribeStatus`.
- [x] 5.3 Add `<script src="/common.js">` before the page script in `index.html` and `monitor.html`.

## 6. Verification

- [x] 6.1 `cargo build` + `cargo test` (29) pass. Seeded an old-schema DB and ran migration 006: timestamps re-encode exactly (julianday parses chrono RFC3339 incl. nanoseconds/offset/`Z`; round-to-nearest-ms), columns drop; live app returns identical JSON (RFC3339 timestamps, protocol/endpoint from config).
- [x] 6.2 DB shrinks: 20k rows 4.38 MB → 1.73 MB (60%) after column drops + epoch timestamps + VACUUM.
- [x] 6.3 Dashboard list now issues a fixed 2 queries (`get_current_status` + `get_uptime_24h_all`) regardless of monitor count (was 2N).
- [x] 6.4 `EXPLAIN QUERY PLAN` confirms uptime + series use `idx_probe_results_monitor_covering` (index-only) over the integer `checked_at`.
- [~] 6.5 Backend verified live (HTTP monitor: live update + history/series/uptime + relative times). Full browser-visual pass of the timeline/traceroute detail views not done here (no browser); JS syntax-checked, data paths confirmed.
- [x] 6.6 Sync the `result-storage` main spec via this change's delta at archive time.

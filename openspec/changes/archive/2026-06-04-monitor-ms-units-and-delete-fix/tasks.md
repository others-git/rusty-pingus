## 1. Millisecond Fields & Migration (monitors + config)

- [x] 1.1 In `src/monitors/mod.rs`, rename `interval_secs`/`timeout_secs` to `interval_ms`/`timeout_ms` on `HttpMonitorConfig`, `TcpMonitorConfig`, `IcmpMonitorConfig`, and update `MonitorConfig::interval_secs()` accordingly (rename to `interval_ms()`)
- [x] 1.2 Update default helpers: `default_interval()` → 60000, `default_timeout()` → 10000 (rename to `default_interval_ms`/`default_timeout_ms`)
- [x] 1.3 Add backward-compatible deserialization that also reads legacy `interval_secs`/`timeout_secs` keys (e.g. `#[serde(alias)]` or a private legacy capture struct) and converts them ×1000 to milliseconds, with `_ms` taking precedence when both are present
- [x] 1.4 On load, detect any value that came from a legacy `_secs` key, log a warning describing the conversion, and rewrite `monitors.toml` in the new `_ms` form (reuse the existing atomic save)
- [x] 1.5 Update `apply_defaults` to compare against the new ms default sentinels and backfill `interval_ms`/`timeout_ms`
- [x] 1.6 Update `DEFAULT_MONITORS_CONFIG` commented examples to use `interval_ms`/`timeout_ms`
- [x] 1.7 In `src/config/mod.rs`, rename `Defaults` fields to `interval_ms`/`timeout_ms` (reading legacy `_secs` aliases ×1000) and update `DEFAULT_CONFIG`

## 2. Scheduler & Probes

- [x] 2.1 In `src/scheduler/mod.rs`, use `Duration::from_millis(interval_ms)` for the per-monitor ticker
- [x] 2.2 In `src/probe/http.rs`, `src/probe/tcp.rs`, `src/probe/icmp.rs`, use `Duration::from_millis(timeout_ms)`
- [x] 2.3 Update probe test fixtures (e.g. `TcpMonitorConfig` in `src/probe/tcp.rs` tests) to use `_ms` fields

## 3. API Validation (ms bounds)

- [x] 3.1 In `src/api/mod.rs`, update `validate_timing` to use ms bounds: `interval_ms >= 5000`, `timeout_ms >= 1000`, `timeout_ms < interval_ms`, with reworded error messages referencing the `_ms` fields
- [x] 3.2 Update the per-protocol validation calls to pass `interval_ms`/`timeout_ms`

## 4. Delete Purges Probe History (bug fix)

- [x] 4.1 Add `db::delete_results(pool, monitor_name) -> Result<u64>` in `src/db/mod.rs` running `DELETE FROM probe_results WHERE monitor_name = ?`, returning rows affected
- [x] 4.2 In `delete_monitor` (`src/api/mod.rs`), after a successful `state.monitors.remove(name)`, call `db::delete_results(&state.pool, name)`; log (do not fail the request) if the purge errors
- [x] 4.3 Confirm 404 behavior is unchanged when the monitor is not in the store (no results purge attempted)

## 5. Frontend (assets)

- [x] 5.1 In `assets/app.js`, rename form fields and request body to `interval_ms`/`timeout_ms`; update default values (e.g. 60000/10000)
- [x] 5.2 In `assets/app.js`, ensure `mapErrors` maps the new `interval_ms`/`timeout_ms` error messages to the correct fields
- [x] 5.3 In `assets/index.html`, update Interval/Timeout labels to "(ms)" and update `min` attributes (interval 5000, timeout 1000)
- [x] 5.4 Verify the delete flow removes the card and that it stays gone after the next 30s poll (covered by backend purge)

## 6. Verification

- [x] 6.1 `cargo build` and `cargo clippy -- -D warnings` pass
- [x] 6.2 `cargo test` passes (update `tests/integration_test.rs` fixtures/assertions for `_ms` fields and the delete-purge behavior)
- [x] 6.3 Smoke test units: start with a legacy `monitors.toml` using `interval_secs`/`timeout_secs`; confirm values load as ×1000 ms, a warning is logged, and the file is rewritten with `_ms` fields
- [x] 6.4 Smoke test add: POST a monitor with `interval_ms`/`timeout_ms`; confirm 201 and correct probing cadence; POST `interval_ms:3000` and confirm 422 with the ms error message
- [x] 6.5 Smoke test delete bug: add a monitor, let it record at least one probe result, delete it, then poll `GET /api/monitors` and confirm it does not reappear and a second DELETE returns 404
- [x] 6.6 Update `README.md` to document millisecond units, validation bounds, and the automatic `_secs` → `_ms` migration

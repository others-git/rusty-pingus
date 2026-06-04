## Why

Monitor `interval` and `timeout` are currently configured in whole seconds, which prevents sub-second timeouts and reads awkwardly against the millisecond response times shown everywhere else in the UI. Separately, deleting a monitor leaves its historical probe results in the database; because the dashboard derives its monitor list from those results, deleted monitors (including the seeded examples) reappear on the next poll and can no longer be deleted — the `DELETE` call returns 404 since the config is already gone.

## What Changes

- **BREAKING**: Monitor timing is configured in **milliseconds**, not seconds.
  - `interval_secs` → `interval_ms`, `timeout_secs` → `timeout_ms` on every monitor type (HTTP/TCP/ICMP).
  - `[defaults]` in `config.toml`: `interval_secs`/`timeout_secs` → `interval_ms`/`timeout_ms`.
  - On load, legacy `_secs` values are auto-converted to milliseconds (×1000) and rewritten, so existing files keep their current cadence with no user action.
  - API request/response bodies use `interval_ms` / `timeout_ms`.
  - Validation bounds (in ms): `interval_ms ≥ 5000`, `timeout_ms ≥ 1000`, `timeout_ms < interval_ms`.
- **Bug fix — deleted monitors reappear**: `DELETE /api/monitors/:name` now also purges that monitor's stored probe results, so the dashboard (which lists monitors from `probe_results`) no longer shows a deleted monitor. This fixes the case where deleting the default/example monitors left undeletable "ghost" cards.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `monitor-config`: Monitor definitions express interval/timeout in milliseconds (`interval_ms`, `timeout_ms`); legacy `_secs` fields are auto-converted ×1000 on load.
- `monitor-crud-api`: Add/list request and response bodies use `_ms` fields; validation bounds are expressed in milliseconds; `DELETE /api/monitors/:name` removes the monitor's stored probe results in addition to its configuration.
- `result-storage`: The system can delete all stored probe results for a named monitor.
- `web-ui`: The add-monitor form expresses interval/timeout in milliseconds, and a deleted monitor is removed from the dashboard and does not reappear on subsequent polls.

## Impact

- `src/monitors/mod.rs` — rename fields to `interval_ms`/`timeout_ms`, default helpers, `DEFAULT_MONITORS_CONFIG` examples, secs→ms load migration, `apply_defaults`.
- `src/config/mod.rs` — `Defaults` fields and `DEFAULT_CONFIG` switch to `_ms`.
- `src/scheduler/mod.rs` — `Duration::from_millis(interval_ms)`.
- `src/probe/{http,tcp,icmp}.rs` — `Duration::from_millis(timeout_ms)`; test fixtures.
- `src/api/mod.rs` — validation bounds/messages in ms; `delete_monitor` calls into the DB to purge results.
- `src/db/mod.rs` — new `delete_results(pool, monitor_name)` query.
- `assets/index.html`, `assets/app.js` — form labels/defaults in ms; delete flow.
- `tests/integration_test.rs` — update fixtures/assertions for ms and delete-purge.
- `README.md` — document millisecond units and the migration.

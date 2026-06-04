## Context

Monitor configs currently store `interval_secs` and `timeout_secs` as `u64` seconds. These flow into `tokio::time::interval(Duration::from_secs(..))` (scheduler) and the per-protocol probe timeouts (`Duration::from_secs(..)`). `[defaults]` in `config.toml` carries optional `interval_secs`/`timeout_secs` that backfill unset monitor values via `monitors::apply_defaults`.

The dashboard's monitor list (`GET /api/monitors`) is derived from the `probe_results` table — `db::get_current_status` returns the latest row per `monitor_name`. The monitor *config* store (`monitors.toml`) and the *results* table are independent. `DELETE /api/monitors/:name` removes the config and stops the probe task but never touches `probe_results`, so historical rows keep a deleted monitor visible on the dashboard. A second delete attempt 404s because the name is no longer in the store.

## Goals / Non-Goals

**Goals:**
- Express and validate all monitor timing in milliseconds (`interval_ms`, `timeout_ms`).
- Auto-migrate existing `_secs` values (×1000) on load so current deployments keep their cadence with zero manual edits.
- Make `DELETE /api/monitors/:name` fully remove a monitor: config + stored probe results, so it disappears from the dashboard and does not reappear.

**Non-Goals:**
- Sub-second intervals/timeouts (validation floor stays at 5000ms / 1000ms).
- Per-probe-result soft delete or archival; deleted monitor results are hard-deleted.
- A general "rename/edit monitor" feature.
- Changing how response times are stored (`response_time_ms` is already in ms).

## Decisions

### 1. Rename fields to `interval_ms` / `timeout_ms` (not "keep name, change meaning")
Renaming makes the unit explicit in every config file, API body, and form, and lets us detect legacy files by the presence of the old keys. Alternative — keeping `_secs` names but interpreting them as ms — was rejected as silently dangerous (a `60` that used to mean 60s would become 60ms).

### 2. Backward-compatible deserialization with ×1000 migration on load
`HttpMonitorConfig`/`TcpMonitorConfig`/`IcmpMonitorConfig` and `Defaults` deserialize both the new `_ms` keys and the legacy `_secs` keys (the latter via `#[serde(alias = ...)]` or an explicit legacy field). On load, if a value arrived through a legacy `_secs` key, it is multiplied by 1000 to become milliseconds. After load, `MonitorStore` rewrites `monitors.toml` in the new `_ms` form (atomic write, same as today), so the migration is one-time and the file ends up canonical. A warning is logged describing the conversion. This mirrors the existing one-time `[[monitors]]`→`monitors.toml` migration pattern.

Implementation note: the cleanest approach is a private deserialization struct that captures both keys, then normalizes to a single `_ms` field on the public type, so the rest of the code only ever sees `interval_ms`/`timeout_ms`.

### 3. Validation bounds in milliseconds: `interval_ms ≥ 5000`, `timeout_ms ≥ 1000`, `timeout_ms < interval_ms`
These preserve today's effective floors (5s / 1s) expressed in ms, keeping behavior identical for existing users. Error messages are reworded to reference the `_ms` fields and bounds.

### 4. Delete purges probe results via a new `db::delete_results`
Add `db::delete_results(pool, monitor_name) -> Result<u64>` running `DELETE FROM probe_results WHERE monitor_name = ?`. The `delete_monitor` handler removes the config from the store first; on success it also calls `delete_results`. Ordering: config removal is the source of truth for "exists" (drives 404), and the results purge is best-effort cleanup — a failure to purge is logged but does not fail the request (the monitor is already gone from config). Alternative — having the dashboard intersect `get_current_status` against the live config list — was rejected because it leaves orphaned rows accumulating forever and doesn't actually clean state.

### 5. `AppState` already carries both `pool` and `monitors`
The delete handler already has `State<AppState>`, so it can call both `state.monitors.remove(..)` and `db::delete_results(&state.pool, ..)` with no signature changes.

## Risks / Trade-offs

- **Migration ambiguity if a file mixes `_ms` and `_secs` keys for the same monitor** → Treat `_ms` as authoritative when present; only convert a `_secs` value when its `_ms` counterpart is absent. Document this.
- **Hard-deleting results loses history if a monitor is re-added with the same name** → Acceptable and matches user intent ("delete" means gone); re-adding starts fresh. Documented as a non-goal to preserve.
- **Breaking config change** → Mitigated by automatic ×1000 conversion + rewrite, so no user action is needed; only hand-rolled tooling that writes `_secs` keys is affected, and those keep working via the alias until the file is rewritten.
- **Default-value sentinel in `apply_defaults`** → Current code backfills only when a monitor value equals the hardcoded default. After switching units, the default sentinels must also move to ms (e.g. 60000/10000) or the backfill logic must change; ensure the comparison uses the ms defaults.

## Migration Plan

1. Ship the renamed `_ms` fields with `_secs` aliases and load-time ×1000 conversion.
2. On first run after upgrade, `monitors.toml` (and `config.toml` defaults) are rewritten in `_ms` form; a warning is logged.
3. Rollback: revert the binary. A `monitors.toml` already rewritten to `_ms` will not parse on the old (seconds-only) binary; document that downgrading requires restoring the pre-upgrade file or dividing values by 1000.

## Open Questions

_None — unit, migration, bounds, and delete-cleanup behavior are decided above._

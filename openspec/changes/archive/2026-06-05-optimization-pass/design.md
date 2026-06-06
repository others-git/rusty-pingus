## Context

`src/db/mod.rs` (~670 lines) is the data layer; `src/api/mod.rs` serves it; `src/monitors/mod.rs` defines the config enum with five `Raw*` deserialization shims; `assets/app.js` (dashboard) and `assets/monitor.js` (detail) each carry their own live-update plumbing. The schema is five migrations; `probe_results` and `traceroute_hops` are the high-volume tables.

Today: `list_monitors` loops over configured monitors issuing `get_latest_status` + `get_uptime` per monitor (2N queries); `checked_at` is RFC3339 **TEXT** read with `strftime`/string comparison on every aggregation; `probe_results` repeats `protocol`/`endpoint` on every row; `traceroute_hops` stores `rtt_us` that always equals `avg_us`; `formatRelative` is copy-pasted into both front-end files.

The user chose: include data migrations, as one comprehensive change.

## Goals / Non-Goals

**Goals:**
- No change to API request/response shapes, config formats, or UI behavior — pure optimization and cleanup.
- Dashboard listing cost independent of monitor count.
- Smaller `probe_results`/`traceroute_*` on disk; faster range/rollup queries (no per-row timestamp parsing).
- Remove duplicated Rust config logic and duplicated front-end helpers.

**Non-Goals:**
- New features, schema for future features, or dependency changes.
- Rewriting the rollup/series algorithms (only their timestamp arithmetic changes).
- Touching the other themes or unrelated UI.

## Decisions

### Decision: Store probe timestamps as integer epoch milliseconds; format to RFC3339 in Rust on read
`probe_results.checked_at` and `traceroute_runs.checked_at` become `INTEGER` epoch **milliseconds**. Row structs read `i64` and serialize via `DateTime::<Utc>::from_timestamp_millis(...).to_rfc3339()`, so history/status/series JSON is byte-for-byte the same. Range filters compare integers; rollup/uptime bucket math uses `ts/1000` instead of `strftime('%s', …)`.
- *Why:* Milliseconds preserve sub-second ordering (traceroute runs every 500 ms; `id` still tiebreaks), shrink the hot tables and the covering index, and drop the per-row `strftime`/`julianday` parsing that dominates large scans. Formatting on read keeps the client contract unchanged.
- *Trade-off:* A one-time migration must re-encode existing TEXT timestamps. The covering index `(monitor_name, checked_at, status, response_time_ms)` is rebuilt over integers.
- *Alternatives:* Epoch **seconds** (smaller still, but loses sub-second ordering for fast monitors — rejected). A generated text column for back-compat (unneeded — we format in Rust).

### Decision: Drop `protocol`/`endpoint` from `probe_results`; supply them from config at the API layer
These are constant per monitor, so storing them per row is pure duplication. The history/status/list endpoints already have `MonitorStore`; they inject the monitor's `protocol`/`endpoint` into responses. The migration drops the two columns and `VACUUM`s.
- *Why:* Removes two TEXT fields from every row of the largest table. The detail page only needs `protocol` (to choose its view), which the API supplies from config.
- *Trade-off:* History rows for a since-deleted monitor can't recover protocol/endpoint from config; they fall back to a neutral value. Acceptable — those rows are normally purged with the monitor.
- *Alternative:* A `monitors` dimension table joined on read (more normalized, but adds a join and a table for data the app already holds in memory — rejected as heavier than needed).

### Decision: Drop the redundant `traceroute_hops.rtt_us` column
`rtt_us` is written as a copy of `avg_us` and never read (the range query uses `avg_us`). Remove the column and its bind.
- *Why:* One fewer integer per hop row on the highest-frequency table; zero behavior change.

### Decision: One batched dashboard query instead of per-monitor fan-out
Replace the loop with `get_current_status` (already a single `MAX(id) GROUP BY monitor_name` query) plus one batched 24 h uptime query grouped by monitor (from the rollup where possible), then assemble in memory keyed by name.
- *Why:* Turns 2N queries into a small constant. Behavior identical; configured-but-unprobed monitors still render pending.

### Decision: DRY the Rust config plumbing
Add a shared `resolve_timing(interval_ms, interval_secs, timeout_ms, timeout_secs) -> (u64, u64)` used by every `From<Raw*>`, and an `apply_timing_defaults(&mut interval, &mut timeout, &Defaults)` helper invoked from each `apply_defaults` arm (the per-arm match stays, but its body collapses to one call). Keep the traceroute interval floor applied after.
- *Why:* Removes five copies of the same resolution and six copies of the same default-application block; one place to change timing semantics.

### Decision: Extract shared front-end helpers
Add `assets/common.js` exposing `formatRelative` and a small `subscribeStatus({onUpdate, onError})` helper that wraps the EventSource subscribe + poll fallback. `app.js` and `monitor.js` call it; both pages add one `<script src="/common.js">` before their page script.
- *Why:* `formatRelative` is identical in both; the SSE/poll setup is near-identical. One source of truth, smaller page scripts.
- *Trade-off:* One extra request (cheap, cached). The detail page's extra live logic (timeline-value gating, traceroute) stays in `monitor.js`, layered on the shared subscribe.

## Risks / Trade-offs

- **Migration correctness on existing data** → Re-encoding TEXT→epoch must be exact. Mitigation: convert with `CAST((julianday(checked_at) - 2440587.5) * 86400000 AS INTEGER)` (ms since Unix epoch), do it inside a transaction per table, and spot-check a few rows before dropping the old column. Keep the migration idempotent/guarded.
- **Migration time / lock on large DBs** → Re-encoding + `VACUUM` can be slow and holds a write lock. Mitigation: it runs once at startup migration; document the one-time cost. (If it proves too heavy, a follow-up could background it — out of scope here.)
- **Dropped protocol/endpoint for orphan history** → Falls back to neutral values for rows whose monitor no longer exists. Acceptable; such rows are usually purged on delete.
- **SQLite `DROP COLUMN` support** → Requires SQLite ≥ 3.35 (bundled sqlx sqlite is newer). Mitigation: verify at build; otherwise rebuild-table fallback.
- **Front-end load order** → `common.js` must load before `app.js`/`monitor.js`. Mitigation: place the include immediately before the page script in both HTML files; keep it dependency-free.

## Migration Plan

New migration `006_compact_storage.sql` (additive then destructive, in one transaction per table):
1. `probe_results`: add `checked_at_ms INTEGER`; `UPDATE` it from `checked_at`; drop `checked_at`, `protocol`, `endpoint`; rename `checked_at_ms`→`checked_at`; recreate the covering index over the integer column.
2. `traceroute_runs`: same timestamp re-encode.
3. `traceroute_hops`: drop `rtt_us`.
4. `VACUUM` to reclaim freed space.

Rollback is restoring from backup (a destructive migration); the change ships with the embedded assets and a release rebuild. No config or API changes, so clients need no update.

## Open Questions

- Should the timestamp migration be split from the column-drops so each can be reverted independently, or kept as one `006`? Proposed: one migration, transaction-guarded per table.
- Is a neutral fallback (`""`/`"unknown"`) acceptable for protocol/endpoint on orphaned history rows, or should the history endpoint omit those fields when the monitor is gone? Proposed: supply from config when present, neutral fallback otherwise.
- Worth adding a lightweight benchmark/`EXPLAIN QUERY PLAN` check to confirm the index-only path after re-encoding? Proposed: a manual verification task, not automated.

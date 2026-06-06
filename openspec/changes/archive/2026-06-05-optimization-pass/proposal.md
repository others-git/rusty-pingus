## Why

The codebase has grown several monitor types and a live UI quickly, leaving some accumulated inefficiency: the dashboard issues a per-monitor query fan-out, repeated config/validation logic invites drift, two front-end files duplicate the same helpers, and the highest-volume tables store timestamps and per-monitor-constant fields inefficiently — which matters most for the new high-frequency traceroute data. This pass makes the app run better and shrink on disk without changing observable behavior.

## What Changes

**Backend performance**
- Replace the dashboard's per-monitor query fan-out (`list_monitors` calling `get_latest_status` + `get_uptime` in a loop — 2N queries) with a small fixed number of batched queries, so listing cost no longer scales with monitor count.

**Code smells / DRY (Rust)**
- Collapse the repeated `interval_ms`/`timeout_ms` resolution across the five `Raw*MonitorConfig` → `From` impls and the six near-identical arms of `apply_defaults` into a shared helper.
- Factor the repetitive per-variant logic in `validate_monitor` where it doesn't aid clarity.

**DRY (frontend)**
- Extract the verbatim-duplicated `formatRelative` and the near-duplicated SSE-subscribe/poll/`_applyLiveStatus` logic from `app.js` and `monitor.js` into one shared asset both pages load.

**Database space (includes data migrations)**
- Drop the redundant `traceroute_hops.rtt_us` column (it duplicates `avg_us`) and read the representative latency from `avg_us`.
- Store probe timestamps as an **integer epoch** instead of RFC3339 **TEXT** in `probe_results` (and `traceroute_runs`), shrinking the hottest tables and removing `strftime` parsing from range/rollup queries. The history/status APIs continue to return RFC3339 strings (contract unchanged).
- Stop persisting `protocol`/`endpoint` on every `probe_results` row (they are constant per monitor); derive them from the monitor config at the API layer. Reclaim space with a one-time migration + `VACUUM`.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- **result-storage** — the on-disk representation of probe results becomes more compact (integer-epoch timestamps; no per-row duplication of per-monitor-constant fields; no redundant traceroute RTT column), while the existing persistence/query/response contract (fields and RFC3339 timestamps returned by history/status/series) is preserved. A migration re-encodes existing rows.

## Impact

- **Backend (`src/`):** `db/mod.rs` (timestamp encoding across insert/history/uptime/series/rollup/prune; batched dashboard query; drop `rtt_us`), `api/mod.rs` (batched `list_monitors`; inject `protocol`/`endpoint` from config into history/status responses), `monitors/mod.rs` (DRY the `From`/`apply_defaults`/validation), `scheduler/mod.rs` (insert call sites).
- **Database:** new migration — convert `checked_at` TEXT→INTEGER epoch in `probe_results` and `traceroute_runs`, drop `probe_results.protocol`/`endpoint`, drop `traceroute_hops.rtt_us`, rebuild affected indexes, and `VACUUM`. Existing data is migrated in place.
- **Frontend (`assets/`):** new shared `common.js` (or similar) for `formatRelative` + SSE/poll helper; `app.js` and `monitor.js` consume it; `index.html`/`monitor.html` include it.
- **No API request/response shape change**, no config-format change, no new dependencies.
- **Specs:** `result-storage`.

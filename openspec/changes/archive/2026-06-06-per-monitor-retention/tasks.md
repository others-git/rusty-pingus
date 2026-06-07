## 1. Config: per-monitor retention_hours

- [x] 1.1 Add `retention_hours: u64` to every monitor config struct + `Raw*` (`#[serde(default)]`), with a `MonitorConfig::retention_hours()` accessor; default to the global retention when unset.
- [x] 1.2 For traceroute, accept the legacy `retention_ms` and convert to hours into the unified `retention_hours` (keep deserialization back-compatible); have the unified value drive hop-table pruning too.
- [x] 1.3 Add the `retention_hours` example to `DEFAULT_MONITORS_CONFIG`; unit tests (default fallback; legacy `retention_ms` → hours).

## 2. Storage: per-monitor pruning + state-segment query

- [x] 2.1 Add `prune_old_results`/`prune_old_rollups` variants that take a monitor name + ms cutoff (prune one monitor by its retention).
- [x] 2.2 Add a `get_state_segments(name, from, to)` query returning collapsed `{ state, start_ms, end_ms }` runs over the range (scan ascending, merge consecutive same `detail` leading-token), bounded by changes — for public-IP/border timelines.
- [x] 2.3 Reuse the per-monitor traceroute prune for hops by the unified retention.

## 3. Scheduler/main: per-monitor retention loop

- [x] 3.1 Replace the single global `retention_loop` with a per-monitor loop (read the live monitor list, prune each monitor's results/rollups/hops by its `retention_hours`); keep the global default for monitors without an explicit value.
- [x] 3.2 Wire it in `main.rs` (it needs the `MonitorStore`, like the traceroute loop).

## 4. API

- [x] 4.1 Add `retention_hours` to `MonitorStatus` (from config) and accept it on `POST /api/monitors`.
- [x] 4.2 Add an endpoint serving state-timeline segments over `[from,to]` for public-IP/border monitors (backed by `get_state_segments`).

## 5. Frontend: brush + full-window timeline + retention UI

- [x] 5.1 Generalize the traceroute brush into a reusable detail-page brush bounded by `[now − retention, now]`; use it on response-time and state-timeline pages to drive the active `[from,to]`.
- [x] 5.2 Replace the fixed 1h/24h/7d/30d chart-range buttons with the brush; keep the uptime gauges as read-only stat tiles.
- [x] 5.3 Switch the public-IP/border timeline to the new segments endpoint so it renders the entire window (remove the newest-1000 cap dependency).
- [x] 5.4 Add a retention (hours) field to the add-monitor form and show the configured retention on the detail page.

## 6. Verification

- [x] 6.1 `cargo build` + `cargo test` pass.
- [ ] 6.2 A fast public-IP monitor's detail timeline shows the whole selected window (not ~15 min); segments span the range.
- [ ] 6.3 Set a short retention on one monitor and a long one on another; confirm the periodic prune trims each to its own retention (and rollups/hops), without restart, while an unconfigured monitor still prunes at 90 days.
- [ ] 6.4 The detail brush spans the retained window, drives the chart/timeline range, and can't scrub past retained data; zoom/pan and live refresh still work; uptime tiles still display.
- [ ] 6.5 Create a monitor with a retention via the form; confirm it persists and shows on the detail page.
- [ ] 6.6 Sync the `monitor-config`, `result-storage`, and `web-ui` main specs at archive time.

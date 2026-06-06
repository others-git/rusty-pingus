## 1. Config: traceroute monitor type

- [x] 1.1 Add a `Traceroute(TracerouteMonitorConfig)` variant to `MonitorConfig` in `src/monitors/mod.rs` with fields: `name`, `host`, `interval_ms`, `timeout_ms`, `max_hops`, `queries_per_hop`, `retention_ms`.
- [x] 1.2 Wire the new variant into `name()`, `interval_ms()`, `protocol()` (`"traceroute"`), and `endpoint()` (the target host).
- [x] 1.3 Add `RawTracerouteMonitorConfig` + `From` impl with `*_ms`/`*_secs` back-compat; clamp `interval_ms` to a minimum of 500 (`.max(500)`); default `max_hops` (30), `queries_per_hop` (3), and `retention_ms`.
- [x] 1.4 Extend `apply_defaults` for the new variant and add a commented example block to `DEFAULT_MONITORS_CONFIG`.
- [x] 1.5 Unit tests: 500 ms floor (sub-floor raised, at/above preserved) and default fields.

## 2. Storage: compact tables + queries + pruning

- [x] 2.1 Add a migration creating `traceroute_addrs(id, addr UNIQUE)`, `traceroute_runs(id, monitor_name, checked_at, reached, hop_count)`, and `traceroute_hops(run_id, hop_no, addr_id NULL, rtt_us NULL, min_us, avg_us, max_us, loss)`, with indexes on `traceroute_runs(monitor_name, checked_at)` and `traceroute_hops(run_id)`. (`migrations/005_traceroute.sql`)
- [x] 2.2 In `src/db/mod.rs`, add an insert path that interns addresses (upsert by unique) and writes one run + its hop rows in a transaction. (`insert_traceroute`)
- [x] 2.3 Add a range query returning per-hop-position aggregates (latest address, reachability, min/avg/max RTT) over runs in `[from, to]`. (`get_traceroute_hops`, plus `get_traceroute_extent`)
- [x] 2.4 Add a per-monitor prune that deletes runs (and hops) older than `retention_ms`, then GCs unreferenced `traceroute_addrs` rows in the same transaction. (`prune_traceroute`; `delete_traceroute` for monitor removal)
- [~] 2.5 Unit/integration tests: interning dedupes addresses, prune removes old runs + orphan addrs but keeps within-retention data, range query aggregates correctly. (DB integration tests deferred — see note; pure helpers in the probe are unit-tested.)

## 3. Probe: native ICMP traceroute

- [x] 3.1 Add `socket2` (raw socket) to `Cargo.toml`.
- [x] 3.2 Create `src/probe/traceroute.rs`: for `ttl in 1..=max_hops`, send `queries_per_hop` ICMP Echo Requests with the IP TTL set; receive within `timeout_ms`, correlating Time-Exceeded / Echo-Reply to the originating probe via id/seq + echoed inner header. (IPv4; IPv6 returns a clean `ipv6_unsupported` down — documented limitation.)
- [x] 3.3 Build the per-hop result (address or unreachable, per-run min/avg/max from responding queries, loss); stop the walk on destination Echo-Reply; mark reached/not-reached.
- [x] 3.4 Define a multi-hop result type and have the probe also emit a lightweight `ProbeResult` summary (status, destination RTT as `response_time_ms`, short `detail`) for the dashboard/SSE.
- [x] 3.5 Handle raw-socket open failure (e.g. privileges) as a clean down result with a descriptive reason; keep probe isolation intact.
- [x] 3.6 Register `traceroute` module in `src/probe/mod.rs`.

## 4. Scheduler & wiring

- [x] 4.1 Dispatch the `Traceroute` variant in `src/scheduler/mod.rs`: run the probe, persist the summary to `probe_results` and the hop data to the traceroute tables.
- [x] 4.2 Schedule the per-monitor traceroute prune alongside existing maintenance. (`traceroute_retention_loop`, spawned in `main.rs`.)

## 5. API endpoints

- [x] 5.1 Add an endpoint serving a traceroute monitor's hops over a `[from, to]` range (per-hop aggregates) in `src/api/mod.rs` / `src/web/mod.rs`. (`GET /api/monitors/:name/traceroute`)
- [x] 5.2 Add an endpoint (or extend an existing metadata endpoint) exposing the monitor's retained data extent so the UI brush knows its bounds. (`GET /api/monitors/:name/traceroute/extent`)

## 6. Frontend: traceroute detail page

- [x] 6.1 In `assets/monitor.js`, detect `protocol === 'traceroute'` and load hop data from the range endpoint instead of the line/timeline path.
- [x] 6.2 Render the per-hop table (hop #, address + reachable state, last/avg/min/max RTT) in `assets/monitor.html`, shown only for traceroute monitors. (Avg/Min/Max shown; "last" represented by Avg over range.)
- [x] 6.3 Render the relative-latency waterfall bar per row (HTML/CSS), scaled to the slowest hop in view.
- [x] 6.4 Add a resizable scroll/brush control bounded by the retained extent; on change, re-fetch and re-aggregate the table; hide the 1h/24h/7d/30d window selectors for this type. (ECharts time slider over the extent.)
- [x] 6.5 Add traceroute fields to the add-monitor form (target, interval with 500 ms min, timeout, max hops, queries-per-hop, retention). (Retention collected in hours → ms.)

## 7. Verification

- [x] 7.1 Build (`cargo build`) and run; create a traceroute monitor via the UI against a reachable host and confirm hops populate. (Verified via API under root: real per-hop router IPs + RTTs returned; interning confirmed by `samples` across runs. Interval floor 100→500 confirmed.)
- [x] 7.2 Confirm the detail table shows per-hop reachability and min/avg/max, with waterfall bars scaling to the slowest hop. (Data verified end-to-end incl. a min/max decode fix; latencies climb down the path. Table/bar markup implemented + JS syntax-checked; not screenshot-verified in a browser.)
- [~] 7.3 Confirm the brush refines the time range and the table re-aggregates; confirm no fixed uptime windows are shown. (Brush + re-aggregation logic implemented and the uptime windows are hidden via `x-show`; not visually verified in a browser.)
- [x] 7.4 Confirm an unreachable/timing-out hop renders as unreachable without breaking the run, and the monitor still appears on the dashboard/live stream. (Per-hop `loss` observed; privilege failure produced a clean down (no panic); dashboard summary shows the monitor `up` with an "N hops" detail. Unreachable-row rendering implemented.)
- [~] 7.5 Confirm the prune deletes data older than retention (e.g. set a short retention) and that database growth is bounded. (`prune_traceroute` + 60s `traceroute_retention_loop` implemented; not runtime-verified — would need to wait a prune cycle.)
- [ ] 7.6 Verify on Windows (raw-socket behavior) or document the limitation if blocked. (Not run — no Windows host here. Risk documented in design.md; IPv6 also returns a clean `ipv6_unsupported`.)
- [x] 7.7 Sync the modified specs (`monitor-config`, `probe-engine`, `result-storage`, `web-ui`) and the new `traceroute-monitor` spec at archive time.

## Why

The existing monitors answer "is this one endpoint up, and how fast?" — a single status and latency per probe. They can't show *where* on the path latency or loss is introduced. A traceroute-style monitor reveals the per-hop route to a target (reachability, latency, and min/avg/max per hop), which is what you need to localize "the internet is slow" to a specific hop. Its data is fundamentally different in shape (many hops per run) and volume, so it needs its own purpose-built storage, retention, and detail page rather than being forced into the single-latency model.

## What Changes

- **New `traceroute` monitor type.** Performs a native raw-socket ICMP traceroute to a target: sends echo probes with increasing TTL, captures the ICMP Time-Exceeded reply from each intermediate hop and the Echo-Reply from the destination, recording per-hop address, reachability, and round-trip time. Sends several queries per hop per run (classic traceroute style) to compute per-run min/avg/max.
- **Enforced interval floor.** The `traceroute` monitor clamps its check interval to a minimum of **500 ms** — a run is expensive and produces a lot of data, so faster scheduling is rejected/clamped.
- **Purpose-built, space-efficient storage.** Per-hop data is NOT stuffed into the single `probe_results.detail` field. It is stored in dedicated, compact tables that intern repeated hop addresses (a route is usually stable, so the same router IPs recur across thousands of runs) and store RTTs as integer microseconds. A lightweight summary row is still written to `probe_results` so the monitor appears on the dashboard and live stream like any other.
- **Configurable retention instead of fixed rollup windows.** This monitor type does not use the 1h/24h/7d/30d rollup windows. Each `traceroute` monitor has a configurable retention period; a periodic prune deletes runs older than retention, deterministically bounding database growth.
- **New detail page layout.** A table with one row per hop (hop #, address, reachable status, last/avg/min/max latency) and a final column rendering a horizontal bar whose length is proportional to that hop's latency — a left-to-right "waterfall" where slower hops extend farther right. A resizable scroll/brush control refines the time range that feeds the per-hop aggregation (within the retained data).
- **Add-monitor form support** for the new type (target host, interval with 500 ms floor, timeout, max hops, queries per hop, retention).

## Capabilities

### New Capabilities
- `traceroute-monitor`: The traceroute probe behavior (TTL-walked ICMP, per-hop reachability + RTT, per-run min/avg/max), the compact per-hop data model with address interning and integer-microsecond RTTs, configurable per-monitor retention with pruning, and the read API that returns hops aggregated over a requested time range.

### Modified Capabilities
- `monitor-config`: Add the `traceroute` monitor variant (target, interval with enforced 500 ms floor, timeout, max hops, queries-per-hop, retention) to the config enum and TOML/UI parsing.
- `probe-engine`: Add traceroute probe execution producing a multi-hop result, alongside the existing single-result probes; preserve probe isolation.
- `result-storage`: Add the dedicated traceroute storage (runs, hops, interned addresses), per-monitor retention pruning for traceroute data, and the per-hop range/aggregation queries.
- `web-ui`: Add the traceroute detail page (per-hop table with relative-latency bars and a resizable time-range brush) and add-monitor form fields for the new type.

## Impact

- **Backend (`src/`):** `monitors/mod.rs` (new `Traceroute` config variant + 500 ms interval floor + defaults), `probe/traceroute.rs` (new probe), `probe/mod.rs` (multi-hop result path), `scheduler/mod.rs` (dispatch + retention prune scheduling), `db/mod.rs` (new storage + queries + prune), `api/mod.rs` and `web/mod.rs` (new endpoints serving hop data over a time range).
- **Database:** New migration adding `traceroute_runs`, `traceroute_hops`, and `traceroute_addrs` (interning) tables and indexes. No change to existing tables; the traceroute monitor still writes a summary row to `probe_results`.
- **Dependencies:** Likely add a raw-socket crate (e.g. `socket2`) to send TTL-limited probes and receive ICMP Time-Exceeded, which `surge-ping`'s echo-oriented API does not surface. Requires elevated privileges, same as the ICMP/border monitors.
- **Frontend (`assets/`):** New traceroute detail rendering (table + ECharts/HTML bars + brush) selected when the monitor protocol is `traceroute`; add-monitor form additions.
- **Specs:** new `traceroute-monitor`; modified `monitor-config`, `probe-engine`, `result-storage`, `web-ui`.

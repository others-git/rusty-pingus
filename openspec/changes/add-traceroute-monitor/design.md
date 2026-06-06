## Context

Existing monitors (HTTP/TCP/ICMP/publicip/border) each produce one `ProbeResult` per cycle — a single status, one optional `response_time_ms`, and an optional `detail` string — stored as one row in `probe_results`. Aggregation for the detail page is served by minute-resolution rollups over fixed windows (1h/24h/7d/30d).

A traceroute monitor breaks that model: one run yields up to ~30 hops, each with its own address, reachability, and RTT, and (for per-run min/avg/max) several queries per hop. At the enforced 500 ms interval floor that is on the order of tens of hop-samples per second per monitor — far more rows than any existing monitor. The route is normally stable, so the same handful of router addresses recur across every run.

Native traceroute requires sending probes with an explicit IP TTL and receiving ICMP **Time-Exceeded** messages from intermediate routers. `surge-ping` (used by the ICMP/border monitors) is built around Echo-Request/Echo-Reply and does not surface Time-Exceeded from third-party hosts, so a lower-level raw socket is needed. Like ICMP/border, this requires elevated privileges.

Constraints: keep the database small (explicit user goal); keep existing tables and monitors untouched; the new monitor must still appear on the dashboard and live stream like every other monitor.

## Goals / Non-Goals

**Goals:**
- A `traceroute` monitor that records per-hop reachability and RTT to a target, with per-run min/avg/max from multiple queries per hop.
- Storage that is as compact as is practical for high-volume, route-stable data.
- Per-monitor configurable retention with deterministic pruning (no unbounded growth).
- A purpose-built detail page: per-hop table with relative-latency "waterfall" bars and a resizable time-range brush over the retained data.
- The monitor integrates with the existing dashboard/SSE/status surfaces with minimal special-casing.

**Non-Goals:**
- Path-change topology diffing / alerting (a future enhancement; we store enough to add it later).
- Reverse-DNS / ASN / geo enrichment of hops (can be layered on later).
- Reusing the 1h/24h/7d/30d rollup machinery for this type — retention + brush replaces it.
- UDP/TCP traceroute variants — ICMP only for this change.
- Fixing the elevated-privilege requirement (inherent to raw sockets; documented, same as ICMP/border).

## Decisions

### Decision: Native raw-socket ICMP traceroute
Open a raw ICMP socket (via `socket2`), and for each hop number `ttl = 1..=max_hops` send `queries_per_hop` ICMP Echo Requests with the IP TTL set to `ttl`. Collect replies within `timeout_ms`: an ICMP **Time-Exceeded** identifies the intermediate hop's address and its RTT; an **Echo-Reply** from the target marks the destination reached and terminates the walk. Hops that produce no reply within timeout are recorded as unreachable (`*`).
- *Why:* Full control over per-hop timing and min/avg/max; cross-platform; reuses the elevated-privilege model already required by ICMP/border; no dependency on a system `traceroute` binary or its locale-specific output.
- *Trade-off:* More code than shelling out, and raw-socket receive must correlate replies to the probe that triggered them (by ICMP id/seq and the echoed inner packet). Windows raw-socket behavior differs and is the riskiest surface.
- *Alternatives:* Shell out to `traceroute`/`tracert` (brittle parsing, slower, inconsistent min/max) — rejected. UDP traceroute (needs raw ICMP receive anyway, Windows-divergent) — deferred.

### Decision: Compact, interned storage in dedicated tables
Three tables, separate from `probe_results`:
- `traceroute_addrs(id INTEGER PK, addr TEXT UNIQUE)` — an interning dictionary so each distinct hop/router address is stored once, not repeated on every run. This is the dominant space win for stable routes.
- `traceroute_runs(id INTEGER PK, monitor_name TEXT, checked_at TEXT, reached INTEGER, hop_count INTEGER)` — one row per run.
- `traceroute_hops(run_id INTEGER, hop_no INTEGER, addr_id INTEGER NULL, rtt_us INTEGER NULL, loss INTEGER)` — one row per hop per run; `addr_id` NULL and `rtt_us` NULL represent a non-responding hop (`*`); RTT stored as **integer microseconds** (no floats). Per-run min/avg/max can be derived from the queries, or stored as a small extra trio of `*_us` columns if we keep only aggregates rather than each query.
- *Why:* Interning + integer microseconds + small integer hop numbers minimizes bytes per sample; keeping it out of `probe_results` avoids bloating the hot status table and the rollup logic. A summary row is still written to `probe_results` (status, destination RTT as `response_time_ms`, a short `detail` like "12 hops · dest reached") so the dashboard, SSE, and status queries work unchanged.
- *Trade-off:* Joins through `traceroute_addrs` on read; we mitigate with indexes (`traceroute_runs(monitor_name, checked_at)`, `traceroute_hops(run_id)`).
- *Alternatives:* JSON hops blob per run (verbose, repeats keys and addresses — rejected on the space goal). Packed binary BLOB per run (most compact but opaque, hard to query/aggregate in SQL — rejected for queryability).

### Decision: Per-monitor retention with periodic pruning (no rollup windows)
Each `traceroute` monitor carries a `retention_ms` (configurable; sensible default, e.g. 24h). A periodic prune — scheduled alongside the existing maintenance — deletes `traceroute_runs` (and their hops) older than `now − retention_ms` for that monitor, then garbage-collects `traceroute_addrs` rows no longer referenced. The detail page's brush selects a sub-range *within* what retention has kept.
- *Why:* Gives the user a direct, predictable knob on database size for this high-volume type; matches the user's request to drop fixed windows in favor of configurable retention + a brush.
- *Trade-off:* No long-term downsampled history beyond retention (acceptable; this type is about recent path behavior). Pruning must be cheap and indexed.
- *Alternatives:* Reuse the global `result-storage` retention policy (too coarse — it's one policy, not per-monitor, and assumes `probe_results` rows) — rejected.

### Decision: Detail page = hop table + relative-latency bars + brush
The detail page (selected when `protocol === 'traceroute'`) renders a table, one row per hop: hop #, address (with reachable/`*` state), last RTT, avg, min, max — all computed over the brushed time range — and a final column with a horizontal bar whose width is proportional to that hop's latency, scaled to the slowest hop in view, producing the left-to-right waterfall the user described. A resizable scroll/brush control above or below the table sets the `[from,to]` range sent to the read API; min/avg/max recompute from the returned aggregation. Reuses the page's live-update plumbing only to refresh the "latest run" indicator, not to thrash the table.
- *Why:* Directly matches the requested layout and keeps aggregation server-side over an explicit range.
- *Trade-off:* A new render path distinct from the ECharts line/timeline; the bar can be plain HTML/CSS (no chart library needed) for the waterfall, with the brush driving a re-fetch.

### Decision: Enforce the 500 ms interval floor at config load
In the `Traceroute` config's `From<Raw…>` conversion, clamp the resolved `interval_ms` to a minimum of 500 (`.max(500)`), mirroring how ICMP clamps `count` to ≥1. Surfaced in the UI as a minimum.
- *Why:* A run is expensive; the floor is a property of the type, enforced where every config path (TOML + UI) funnels through.

## Risks / Trade-offs

- **Raw-socket correlation / privileges** → Receiving ICMP Time-Exceeded and matching it to the originating probe (via the echoed inner IP+ICMP header) is fiddly and privilege-gated. Mitigation: encode a known id/seq per (run,hop,query) and validate the echoed header; document the privilege requirement like ICMP/border; fail the monitor cleanly (down + reason) if the socket can't be opened.
- **Windows raw-socket divergence** → `tracert` semantics differ and raw ICMP receive is more restricted. Mitigation: isolate platform specifics behind the probe module; verify on Windows during implementation; consider a documented fallback if blocked.
- **Data volume despite interning** → Even compact, 500 ms × 30 hops is heavy. Mitigation: integer-microsecond columns, address interning, indexed retention prune; default retention conservative; document the size/retention trade-off.
- **Address-dictionary GC races** → Pruning addrs while a run inserts the same addr. Mitigation: GC only addrs with zero references inside the same transaction as the run delete; inserts upsert-by-unique.
- **Scope creep into topology/alerting** → Explicitly a non-goal; schema retains run/hop granularity so it can be added later without migration churn.

## Migration Plan

- Additive DB migration creating `traceroute_addrs`, `traceroute_runs`, `traceroute_hops` and their indexes; no change to existing tables, so rollback is dropping the new tables.
- New dependency (`socket2`) added to `Cargo.toml`; release build re-embeds assets as usual; the running server must be rebuilt/restarted to pick up the new probe and embedded UI.
- Backward compatible: existing monitors and their data are untouched; the new type is opt-in via config/UI.

## Open Questions

- **Per-run min/avg/max storage**: store each query's RTT (more rows, exact recomputation over any range) vs. store only the per-run min/avg/max trio (far fewer rows, but range aggregation across runs is approximate)? Leaning toward the aggregate trio per (run,hop) for the space goal — confirm during specs.
- **Default retention value** and whether retention is expressed in ms (consistent with other timing fields) or a friendlier unit in the UI (hours/days). Proposed: store `retention_ms`, present hours/days in the form.
- **Destination addressing**: target given as host or IP; resolve once per run or per scheduler cycle? Proposed: resolve per run, record the resolved dest address on the run.
- **Reverse-DNS for hop display**: out of scope now; revisit as enrichment.

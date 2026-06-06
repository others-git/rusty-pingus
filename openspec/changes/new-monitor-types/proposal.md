## Why

Rusty-pingus monitors HTTP/TCP/ICMP — all "is the endpoint up, how fast" checks. Two requested checks don't quite fit that mold and add real diagnostic value, especially for home/self-hosted setups:

- **Public-IP monitor** — tracks your external IP (via checkip.amazonaws.com / icanhazip.com). Tells you when your ISP rotates your IP (useful for DNS, port-forwarding, allow-lists) and whether you have working egress at all.
- **Border monitor** — pings your local gateway *and* an upstream reference, so an outage is localized: is your router/LAN down, or is the ISP/internet down? "Is it me or them."

(A third idea — a traceroute hop-path plotter — is intentionally **out of scope here**: it needs a multi-hop-per-probe data model and a path-graph visualization, so it gets its own change later.)

Both new checks need to record a small piece of per-probe context the current schema can't hold (the observed IP; the border fault classification), so this also adds a generic `detail` field to probe results.

## What Changes

- **`detail` on probe results.** Add a nullable `detail` text column to `probe_results` (and `ProbeResult.detail`), threaded through insert and history. Type-specific, human-readable context: the public IP for a public-IP check; the fault localization for a border check. Existing types leave it null.
- **Public-IP monitor type** (`protocol = "publicip"`). Periodically GETs an IP-echo service (default with fallback), parses the returned address, and records it in `detail`. Up when the service responds with a valid IP; down otherwise. A change in the IP between consecutive checks is detected and surfaced.
- **Border monitor type** (`protocol = "border"`). ICMP-pings the local gateway (auto-detected, overridable) and an upstream reference (default `1.1.1.1`), and classifies the result: `ok`, `isp_down` (gateway reachable, upstream not), or `lan_down` (gateway unreachable). Overall status follows upstream reachability; `detail` carries the localization and both latencies.
- **Add/validate via API + UI.** `POST /api/monitors` accepts and validates the two new types; the add-monitor modal gains tabs/fields for them; the dashboard and detail views surface `detail` (current public IP / border status).

## Capabilities

### New Capabilities

- `public-ip-monitor`: A monitor that records the host's external IP and flags changes.
- `border-monitor`: A monitor that localizes connectivity faults (LAN vs ISP) by probing the gateway and an upstream reference.

### Modified Capabilities

- `result-storage`: probe results gain a nullable `detail` field for per-probe type-specific context.
- `monitor-crud-api`: `POST /api/monitors` accepts and validates `publicip` and `border` monitors.
- `web-ui`: the add-monitor form supports the two new types, and the dashboard/detail views display their `detail`.

## Impact

- `migrations/004_detail.sql` — `ALTER TABLE probe_results ADD COLUMN detail TEXT`.
- `src/probe/mod.rs` — add `detail: Option<String>` to `ProbeResult` (+ constructor/setter); `src/db/mod.rs` — persist/read `detail` (insert, history, current status).
- `src/monitors/mod.rs` — `PublicIp` and `Border` config variants (+ name/interval/timeout, `protocol()`/`endpoint()`).
- `src/probe/publicip.rs`, `src/probe/border.rs` — new probes; `src/scheduler/mod.rs` — dispatch them.
- `src/api/mod.rs` — validation for the new types.
- `assets/index.html` + `assets/app.js` — add-monitor modal tabs/fields + `detail` display; monitor cards/detail show `detail`.
- Gateway auto-detection is platform-specific (best-effort, overridable) — see design.
- No change to series/rollup aggregation (those stay response-time only).

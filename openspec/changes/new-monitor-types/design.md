## Context

Monitors are a `#[serde(tag = "protocol")]` enum (`Http`/`Tcp`/`Icmp`) in `src/monitors/mod.rs`; each variant has a probe in `src/probe/<proto>.rs` returning a `ProbeResult { monitor_name, protocol, endpoint, status: up|down, response_time_ms, failure_reason, checked_at }`, persisted to `probe_results` and surfaced by the dashboard/detail views. `status` is CHECK-constrained to `up`/`down`. There is no field for type-specific output (an observed IP, a fault class). ICMP probing already exists (`probe::icmp`), and the HTTP probe already does GET-with-timeout — both new types build on these.

## Goals / Non-Goals

**Goals:**
- Add a public-IP monitor (records external IP, flags changes) and a border monitor (localizes LAN vs ISP faults), reusing existing HTTP/ICMP probing.
- Add a generic per-probe `detail` field so these (and future types) can record context.
- Validate + add them via the API and the add-monitor UI; display their `detail`.

**Non-Goals:**
- Traceroute / hop-path plotting (separate change — different data model + viz).
- Alerting/notifications on IP change (just surface it; alerting is its own feature).
- Aggregating `detail` into the series/rollup (those stay response-time only).
- Robust cross-platform default-gateway discovery beyond best-effort + manual override.

## Decisions

### 1. `detail: Option<String>` on results (migration 004)
`ALTER TABLE probe_results ADD COLUMN detail TEXT` (nullable; existing rows/types stay null). Add `detail` to `ProbeResult` and thread it through `db::insert_result`, `get_history`, and `get_latest_status`/`get_current_status`. Keep `up`/`down` semantics; `detail` is purely descriptive context, not status. Series/rollups ignore it.

### 2. Public-IP monitor (`protocol = "publicip"`)
Config: `name`, optional `url` (default `https://checkip.amazonaws.com`, fallback `https://icanhazip.com`), `interval_ms`, `timeout_ms`. Probe: GET the service (try default, then fallback on failure), parse the body (trim whitespace) and validate it parses as an `IpAddr`. **Up** → record the IP in `detail` and `response_time_ms`; **down** → request failed / non-2xx / unparseable body, with `failure_reason`. **Change detection:** read the most recent prior `detail` for this monitor (one `get_latest_status`-style lookup); if it differs from the new IP, note the change (e.g. `detail = "203.0.113.7 (changed from 198.51.100.4)"`) so it's visible in history. `endpoint()` = the IP service URL.

### 3. Border monitor (`protocol = "border"`)
Config: `name`, optional `gateway` (IP; auto-detect if absent), `upstream` (IP/host, default `1.1.1.1`), `interval_ms`, `timeout_ms`. Probe: ICMP-ping `gateway` and `upstream` (reuse `probe::icmp` internals), then classify:
| gateway | upstream | status | detail |
|---|---|---|---|
| up | up | `up` | `ok` (+ both RTTs) |
| up | down | `down` | `isp_down` — gateway reachable, upstream not |
| down | (either) | `down` | `lan_down` — local gateway unreachable |
Overall `status` = up iff upstream reachable; `response_time_ms` = upstream RTT (the meaningful "internet latency"); `detail` carries the classification + gateway/upstream RTTs. `endpoint()` = `"<gateway> → <upstream>"`.

**Gateway auto-detection** is best-effort and platform-specific (Linux: parse `/proc/net/route`; Windows: `GetBestRoute`/`ip` equivalent; macOS: `route -n get default`). If detection fails and no `gateway` is configured, the monitor reports `down`/`lan_down` with a clear `failure_reason` ("gateway not configured / not detected") rather than guessing. Recommend documenting manual `gateway` config as the reliable path; treat the "next hop to the ISP" precisely (traceroute hop 2) as a refinement deferred with the traceroute change — `upstream` (a reference like 1.1.1.1) is the pragmatic stand-in.

### 4. Reuse, not duplicate, the ICMP probe
Factor the single-host ICMP ping in `probe::icmp` into a reusable helper (`ping_once(host, timeout) -> Result<rtt_ms>`) that both the ICMP probe and the border probe call, so border doesn't reimplement raw-socket logic. Border inherits the same CAP_NET_RAW/privilege requirement as ICMP (documented).

### 5. API validation + UI
- Validation (`src/api/mod.rs`): `publicip` → if `url` given, must be http(s). `border` → `gateway` (if given) and `upstream` must be valid IP/host. Standard name rules; no interval/timeout bounds (already removed).
- UI: two new protocol tabs in the add-monitor modal ("Public IP", "Border") with their fields (publicip: optional URL; border: optional gateway, upstream). Dashboard cards / detail view show `detail` (current public IP; border classification) where present.

## Risks / Trade-offs

- **Gateway auto-detection portability** — three OS code paths, each fragile; mitigated by manual override + a clear failure message. Keep auto-detect best-effort.
- **Public-IP service availability / rate limits** — mitigated by a fallback service; a single failure marks down (transient, recovers next interval).
- **`detail` is free-form** — intentionally human-readable, not structured; if future features need to query it (e.g. "show IP history"), revisit with a typed column.
- **Border reuses ICMP privileges** — on Linux needs CAP_NET_RAW/root, same as ICMP monitors; documented.
- **Status still binary** — fault localization lives in `detail`, not new status values, to avoid touching the `up`/`down` CHECK and all status-aware queries/UI.

## Open Questions

- Default `upstream` (1.1.1.1) vs letting the border monitor derive the true ISP next hop — deferred to the traceroute change.
- Whether IP-change should eventually raise a notification (out of scope; needs an alerting capability).

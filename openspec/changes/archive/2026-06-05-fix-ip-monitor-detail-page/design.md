## Context

`assets/monitor.js` drives the detail page. `selectWindow(label)` sets `windowTo = Date.now()` and `windowFrom = windowTo − secs`, then `loadRange(windowFrom, windowTo)` sends those bounds as ISO strings to `/history` and `/series`. Probe rows are stored with **server-clock** RFC3339 timestamps; the history query filters `checked_at BETWEEN from AND to` by string comparison.

Observed failure: the server clock is ~4h40m ahead of the browser clock (WSL2 host/guest drift). The browser asks for `[Date.now()−24h, Date.now()]`, which ends ~4h40m *before* the newest probe, so the ranged query returns `[]` → empty chart and, for public-IP, `_computeIpSummary([])` → "no data". The dashboard and the header badge are unaffected because they read the latest snapshot (`/history?limit=1`-style, no time filter), which is skew-immune. Uptime gauges also work because `get_uptime` windows are computed on the server clock.

The `live-detail-page` change then added a per-probe auto-refresh that re-ran the same skewed query every second and fully re-rendered the chart — producing the reported flashing without ever showing data.

Constraints: keep it frontend-only if possible (the previous change was); the newest probe timestamp is already returned by existing endpoints; do not regress the response-time detail pages.

## Goals / Non-Goals

**Goals:**
- The detail chart/timeline shows data correctly even when the client and server clocks differ.
- Public-IP detail page does not flash and does not pointlessly reload per probe.
- Public-IP detail page is purpose-built (no response-time framing).
- No backend, API, schema, or dependency changes.

**Non-Goals:**
- Fixing the machine's clock skew itself (environmental; the app must be robust to it).
- Changing live-update behavior for response-time monitors (ICMP/HTTP/TCP) beyond the windowing fix.
- Server-side relative-window API (considered below, not required for the fix).

## Decisions

### Decision: Anchor time windows to the latest server-recorded probe timestamp, not `Date.now()`
Introduce a `serverNow()` notion = the most recent probe `checked_at` known to the client (captured from the initial `/history` load and updated from live events), falling back to `Date.now()` only when there is no data. `selectWindow` and any derived range computation use this anchor for `windowTo`.
- *Why:* Both window bounds become server-clock-derived, matching the stored timestamps, so the ranged query selects the data regardless of skew. Frontend-only; uses data already on hand.
- *Trade-off:* The chart's right edge is the last probe time rather than wall-clock "now" — for an actively probing monitor these coincide; for a stopped monitor, anchoring to its last data is actually more useful than an empty tail.
- *Alternatives considered:*
  - *Server-relative window param* (`/history?window_secs=N`, server computes `to = now`): most correct long-term and also fixes any future client, but adds API surface and backend work. Deferred; the client anchor fixes the reported bug with less risk and no API change.
  - *Tolerant timestamp comparison / widening the window by a slop factor:* hacky and unbounded (skew can be arbitrary).

### Decision: Suppress per-probe chart reloads for state-timeline monitors
For `timelineKind` of `publicip`/`border`, the live path updates the header value (`detail`, status) but does **not** call the debounced chart reload on every probe. Trigger a timeline reload only when the broadcast value (the IP / fault class) differs from the currently displayed latest value, plus the existing user-driven paths (window select, zoom/pan, manual refresh).
- *Why:* The tracked value changes rarely; reloading the custom-series timeline each second is what visibly flashes. Gating on actual value change keeps it live where it matters and still stops flashing.
- *Alternative considered:* Disable live updates entirely on these pages — rejected; a header that goes stale after an IP change is the original complaint in reverse.

### Decision: Keep response-time monitors' live updates as-is (minus the windowing fix)
The per-probe live refresh remains for response-time charts (latency genuinely changes each probe), now anchored to server time so it shows data. The non-flashing gate is specific to timeline kinds.

### Decision: Public-IP view carries no response-time framing
Confirm the public-IP branch of the metrics strip and chart header never render latency/response-time values, axis, or labels. The IP strip (current IP / stable for / IP changes / last check) and the IP timeline are the whole story.

## Risks / Trade-offs

- **Anchor goes stale if probing stops** → The right edge sits at the last probe; acceptable and arguably clearer than an empty live tail. Live events advance the anchor while probing continues.
- **"Value changed" detection for timeline reload** → Compare the leading token of `detail` (the IP / fault class) to the last rendered value (the existing `extractState` logic already isolates it); on mismatch, reload.
- **Interaction with the existing zoom/pan refetch** → The windowing change only alters how `windowTo`/`windowFrom` are seeded; the zoom math (relative to those bounds) is unchanged.
- **Two unarchived changes touch the same specs** (`live-detail-page` + this) → This change's spec deltas are additive (`ADDED`) refinements, so they compose with `live-detail-page` at archive time without rewriting its requirements.

## Migration Plan

Frontend-only; ships with the embedded assets. No data migration or rollback steps beyond reverting the asset changes. As before, the running release server must be rebuilt (`cargo build --release`) and restarted to pick up embedded asset changes; a debug build serves assets from disk.

## Open Questions

- Should the server-relative window API be adopted later as the canonical fix (so non-WSL clients are also covered without relying on captured data time)? Proposed as a follow-up, not part of this change.

## Context

The dashboard (`assets/app.js`) is live: it opens an `EventSource` to `GET /api/monitors/stream` and updates each card in place as `StatusUpdate` events arrive, with a 30s poll (`fetchMonitors`) as fallback and for list/long-window reconciliation.

The monitor detail page (`assets/monitor.js`) is the opposite: `init()` runs once on navigation, fetches `/history?limit=100` (to set `currentStatus`, `detail`, and `timelineKind`) plus `/uptime`, then `selectWindow()` loads the active window. After that it only re-fetches on user zoom/pan (a debounced `loadRange`). There is no stream subscription and no poll. This was deliberate — the `live-status-updates` spec required detail pages to stay static.

The observed bug: open `/monitors/IP` right after the monitor was added/renamed (or after a restart), when the active window has no samples yet. The ranged `/history?from&to` query returns `[]`, so `buildSegments([])` is empty and `_computeIpSummary([])` leaves `ipCurrent=null` ("unreachable"), `ipStableFor='—'`, `ipChanges=0`, and `hasData=false` (empty graph). Because the page never refreshes, it stays frozen on that empty snapshot while the dashboard — being live — shows the monitor as up with its IP. The page contradicting the dashboard is the user-visible defect.

Constraints: backend is sufficient as-is (`/api/monitors/stream` + `StatusUpdate` carry `name`, `status`, `response_time_ms`, `last_checked_at`, `failure_reason`, `detail`). The fix is frontend-only. The chart instance is intentionally kept out of Alpine's reactive state, and `loadRange` already guards concurrent refetches with `refetchSeq` and an `isApplying` flag.

## Goals / Non-Goals

**Goals:**
- The detail page stays current as probes arrive, without a manual reload, for every monitor type.
- A page opened on an empty window fills in once data arrives.
- Live refresh never disturbs the user's current zoom/pan or a pending zoom-triggered refetch.
- Distinguish "no data yet" from "down/unreachable" in the detail view's empty states.
- Degrade gracefully to a periodic poll when SSE is unavailable.

**Non-Goals:**
- No backend changes (no new endpoints, payload fields, or schema changes).
- No change to how the chart/timeline renders, zooms, or aggregates beyond what live refresh requires.
- No change to dashboard behavior.
- Not adding per-event incremental point appending to the chart — a debounced reload of the active/visible range is sufficient.

## Decisions

### Decision: Reuse the existing SSE stream, filtered to the viewed monitor
Subscribe to `GET /api/monitors/stream` in `init()` and act only on events where `u.name === this.monitorName`. On a matching event, update the header fields (`currentStatus`, `lastResponseMs`, `lastCheckedAt`, `detail`) immediately from the event payload, then trigger a (debounced) data refresh of the current view.
- *Why:* The stream and payload already exist and the dashboard proves the pattern. Header fields can update instantly from the event without any fetch.
- *Alternative considered:* A dedicated per-monitor stream/endpoint — unnecessary; filtering client-side is trivial and avoids backend work.

### Decision: Refresh the *currently visible* range, debounced, not the whole window
On a live update, re-run `loadRange` for the range currently in view (full active window if the user hasn't zoomed; otherwise the current zoom extent), reusing the existing `refetchSeq`/`isApplying` guards and a short debounce so a burst of fast probes coalesces into one refetch.
- *Why:* Reuses the tested data path and the `notMerge:false` apply that already preserves zoom. Keeps timeline `tlSegments`/legend and line `downIntervals` consistent with a normal load.
- *Trade-off:* Slightly heavier than appending the single new point, but far simpler and correctness-preserving across raw/series/timeline modes. Probe intervals are seconds-plus; debounced reloads are cheap.

### Decision: Live refresh must not reset zoom/pan or fight a pending refetch
Call the live `loadRange` with `resetView=false`, and skip/replace it while a user zoom-triggered refetch is pending (respect the existing `isApplying` guard and the zoom debounce timer).
- *Why:* The existing zoom path uses `notMerge:false` precisely so refetches keep the current extent; reusing it with `resetView=false` preserves position. We must not call `selectWindow` (which resets to full window) on a live tick.

### Decision: Poll fallback mirrors the dashboard
If `EventSource` is unavailable or errors, fall back to a periodic poll (same effect: refresh header + visible range on an interval). Close the `EventSource` on `beforeunload`.
- *Why:* Matches `live-status-updates`' existing fallback guarantee and the dashboard's behavior.

### Decision: Separate "no data" from "unreachable"
Track whether the active window has any samples (e.g. `hasData` / sample count) separately from whether the latest probe failed. In `monitor.html`, the public-IP "Current IP" tile shows a neutral "collecting…/no data" state when the window has no samples, and only shows "unreachable" when there are samples but the latest has no IP (a real down). The chart area already has a "No data in this window" message; ensure the public-IP strip agrees with it.
- *Why:* "unreachable" wrongly implies the IP service failed; for a brand-new monitor the truth is "no data yet". This is the part of the report that most reads as broken.

## Risks / Trade-offs

- **Live reload fighting user zoom/pan** → Reuse `resetView=false` + `isApplying`/`refetchSeq` guards and the existing debounce; never call `selectWindow` on a live tick.
- **Event burst causing refetch storms** → Debounce live-triggered reloads (coalesce rapid probes into one refetch); `refetchSeq` already discards superseded responses.
- **SSE proxies/buffering in some environments** → Poll fallback keeps the page correct even if the stream never connects.
- **Stale-closure / reactivity pitfalls with the chart instance** → Keep the `EventSource` out of Alpine reactive state (as the dashboard does) to avoid proxying a host object.
- **Leaks from unclosed streams on navigation** → Close `EventSource` on `beforeunload`/teardown.

## Migration Plan

Frontend-only; ships with the embedded assets. No data migration, no API change, no rollback steps beyond reverting the asset changes. The reversed spec rule is documented in the `live-status-updates` delta (REMOVED "Detail pages do not auto-update" → ADDED "Detail pages update live with polling fallback").

## Open Questions

- Poll fallback interval: reuse the dashboard's 30s, or shorter on the detail page since it's a focused single-monitor view? (Default: 30s to match the dashboard unless testing shows it feels stale.)

## 1. Live stream subscription (assets/monitor.js)

- [x] 1.1 Add closure-scoped (non-reactive) `eventSource` and a live-refresh debounce timer, alongside the existing chart/closure state.
- [x] 1.2 In `init()` (after the first load completes), open `new EventSource('/api/monitors/stream')`; on each message, parse JSON and ignore events whose `name !== this.monitorName`.
- [x] 1.3 On a matching event, update header fields directly from the payload: `currentStatus`, `lastResponseMs` (`response_time_ms`), `lastCheckedAt` (`last_checked_at`), and `detail`.
- [x] 1.4 Close the `EventSource` on `beforeunload` (and on teardown) to avoid leaks.

## 2. Refresh the visible data without disrupting interaction

- [x] 2.1 On a matching event, trigger a debounced refresh of the currently visible range (full active window if not zoomed, else the current zoom extent), calling `loadRange(..., /* resetView */ false)`.
- [x] 2.2 Guard the live refresh against the existing zoom path: do not fire while `isApplying` is true or a user zoom/pan refetch is pending; rely on `refetchSeq` to discard superseded responses.
- [x] 2.3 Verify the live refresh never calls `selectWindow` (which would reset to the full window) and that zoom/pan position is preserved across a live tick.

## 3. Polling fallback

- [x] 3.1 If `EventSource` is unavailable (or on `onerror` with no connection), start a periodic poll that refreshes the header (re-fetch latest) and the visible range on an interval (default 30s, matching the dashboard).
- [x] 3.2 Ensure the poll and the stream don't double-refresh (stream active → poll is fallback only), mirroring the dashboard's approach.

## 4. Empty-state semantics (no-data vs unreachable)

- [x] 4.1 In `monitor.js`, track "window has no samples" separately from "latest probe failed" (use `hasData`/sample count vs. the latest probe's detail), so the public-IP summary can tell them apart.
- [x] 4.2 In `monitor.html`, update the public-IP "Current IP" tile to show a neutral "collecting…/no data" state when the window has no samples, and reserve "unreachable" for when samples exist but the latest has no IP.
- [x] 4.3 Ensure the chart/timeline empty-state message and the public-IP strip agree (both reflect "no data in this window").

## 5. Verification

- [x] 5.1 Add/rename a monitor and open its detail page immediately; confirm the metrics and chart fill in live without a manual reload.
- [x] 5.2 With the page open, confirm header status/detail (e.g. current public IP) updates within ~1s of a new probe, and that zoom/pan position is preserved across updates.
- [x] 5.3 Confirm a freshly-opened empty window shows "no data/collecting" rather than "unreachable", and that a genuine latest-probe failure still shows "unreachable".
- [x] 5.4 Simulate SSE unavailable (block the stream) and confirm the page still updates via the poll fallback.
- [ ] 5.5 Update the `live-status-updates` and `web-ui` main specs via the change's deltas at archive time (no stale "detail pages stay static" requirement remains).

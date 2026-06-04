## 1. Visible-Range Refetch Plumbing

- [x] 1.1 In `assets/monitor.js`, add state: `chartMode` (`'series' | 'raw'`), `isLoadingDetail` (bool), `refetchSeq` (number for stale-response guarding), and an `isApplying` guard flag
- [x] 1.2 Add a `loadRange(fromMs, toMs)` async method: clamp `to ≤ now`, increment `refetchSeq`, fetch `GET /series?from&to&buckets=300` and sum bucket counts for the true total (the `/history` limit is server-capped at 1000, so it can't be used to detect density); if total ≤ 1000 fetch raw `GET /history?from&to&limit=1000` and use raw, else use the series buckets; ignore the result if `refetchSeq` changed during the await
- [x] 1.3 Add a debounce helper (~300 ms) and a `scheduleRefetch(fromMs, toMs)` that debounces calls to `loadRange`

## 2. Wire Zoom/Pan Callbacks

- [x] 2.1 In the chart's zoom plugin options, add `onZoomComplete` and `onPanComplete` callbacks that read `chart.scales.x.min`/`.max` and call `scheduleRefetch(min, max)` — but no-op while `isApplying` is true
- [x] 2.2 Refactor chart data application so `loadRange` swaps `chart.data.datasets[0].data` and per-point styling, then calls `chart.update('none')` while `isApplying` is set (so the zoom view is preserved and no feedback loop fires)
- [x] 2.3 Build raw-mode points from `/history` rows (`{x: checked_at, y: response_time_ms}`, red where `status==='down'`) and series-mode points from `/series` (existing logic); set `chartMode` accordingly

## 3. Tooltip, Reset, Loading Indicator

- [x] 3.1 Make the tooltip format adapt to `chartMode`: raw mode shows the probe's exact time + response + status/reason; series mode shows avg/min/max + sample count + up-ratio (existing)
- [x] 3.2 Update `resetZoom()` to call `chart.resetZoom()` and reload the active window via `selectWindow(activeWindow)` so resolution returns to the window level
- [x] 3.3 Ensure `selectWindow(label)` routes through the same rendering path and sets the base range; initial load still renders the 24h window
- [x] 3.4 In `assets/monitor.html`, add an "updating…" indicator on the chart card bound to `isLoadingDetail`

## 4. Verification

- [x] 4.1 `cargo build` passes (assets re-embed); `cargo clippy -- -D warnings` passes; `cargo test` passes (no backend changes expected)
- [x] 4.2 Confirm the served `monitor.js` contains the new `loadRange`/`onZoomComplete`/`onPanComplete` wiring and `monitor.html` the loading indicator
- [x] 4.3 Browser check (seed data at a low interval): zoom in and confirm finer buckets load for the visible range; zoom in further and confirm raw probe points appear; pan and confirm the new range loads; rapid zooms are debounced (network panel); reset returns to the active window
- [x] 4.4 Update `README.md` to note the monitor chart loads finer detail (down to raw probes) as you zoom in

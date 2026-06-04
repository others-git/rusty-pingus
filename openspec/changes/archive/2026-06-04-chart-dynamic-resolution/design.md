## Context

After `monitor-detail-revamp`, the detail chart (`assets/monitor.js`) calls `selectWindow(label)`, which fetches `GET /api/monitors/:name/series?from=…&to=now&buckets=300` and renders a Chart.js time-scale line with `chartjs-plugin-zoom` enabled (wheel zoom, drag pan). Zoom/pan currently only transforms the view of already-loaded points; no new data is fetched, so resolution is fixed at the window's `(span / 300)` bucket size. The server already supports arbitrary ranges for both `/series` (bucketed, `buckets` clamped 50–1000, ≥1s buckets) and `/history` (raw rows, `limit` ≤ 1000).

## Goals / Non-Goals

**Goals:**
- On zoom/pan, refetch data for the visible `[from, to]` so the visible window renders at ~300 points.
- When the visible range holds few enough probes, render raw individual points instead of aggregates.
- Debounce refetches; show an unobtrusive loading indicator; keep reset working.

**Non-Goals:**
- Backend/API/schema changes (existing endpoints suffice).
- Client-side caching/prefetching of adjacent ranges, or streaming/live updates.
- Sub-second aggregate buckets (raw mode covers the deep-zoom case instead).

## Decisions

### 1. Refetch on zoom/pan completion via plugin callbacks
Register `chartjs-plugin-zoom` `onZoomComplete` and `onPanComplete` callbacks. On completion, read the visible x-range from `chart.scales.x.min`/`.max` (epoch ms), clamp `to ≤ now`, and trigger a debounced (~300 ms) refetch for that range. Reading the realized scale bounds (rather than the wheel delta) keeps it correct for both zoom and pan. Alternative — refetch on every wheel event — rejected (server spam, jank).

### 2. Source selection: raw probes vs aggregated buckets
On refetch, first request the aggregated series for the visible range: `GET /series?from&to&buckets=300`. The sum of the bucket `count` fields is the **true** number of probes in range, because `/series` scans the whole range with no row limit.
- If that total is **≤ 1000**, the range is sparse enough to render every probe → fetch raw rows (`GET /history?from&to&limit=1000`, which returns all of them) and render raw points (finest detail; each probe visible).
- Otherwise render the aggregated buckets already fetched.

> Note: an earlier approach probed density with `GET /history?limit=1001` and treated "1001 rows" as "too dense". That does not work because the `/history` handler caps `limit` at 1000, so the response can never exceed 1000 and the client would always (incorrectly) choose raw mode and only ever show the 1000 most-recent rows. Counting via `/series` avoids that cap.

This adapts to the monitor's actual probe density without the client needing to know the interval. It costs one request when dense (series only) and two when sparse (series + raw). Alternative — decide purely by a fixed span threshold — rejected because poll intervals vary per monitor (1 s vs 60 s), so a fixed span can't predict row count.

### 3. Prevent refetch feedback loops
Programmatic data/scale updates must not re-trigger `onZoomComplete`/`onPanComplete`. Apply new data with `chart.update('none')` and guard the handlers with an `isApplying` flag (set while updating, cleared after), and ignore callbacks fired during programmatic changes. Only genuine user gestures cause refetches.

### 4. Resolution markers and continuity
Reuse the existing rendering: raw points carry per-point status colouring (red where `status==='down'`); aggregated points stay red where `up_ratio < 1`. The dataset is swapped in place (`chart.data.datasets[0].data = …; chart.update('none')`) so the zoom transform/view is preserved across refetches; only the y/x data densifies. A `mode` flag (`'raw' | 'series'`) drives tooltip formatting.

### 5. Reset and the active window
The reset control calls `chart.resetZoom()` and then reloads the active window's series (`selectWindow(activeWindow)`), restoring the window-level resolution. Clicking a different uptime window still sets the base range as today.

### 6. Loading affordance
An `isLoadingDetail` flag toggles a small spinner/"updating…" label on the chart card while a refetch is in flight, so the densify step is visible without blocking interaction.

## Risks / Trade-offs

- **Extra request when dense** → At most two requests per settled gesture, and only when the visible range exceeds 1000 raw rows; debouncing bounds frequency. Acceptable.
- **Rapid zoom gestures** → Debounce + dropping stale responses (track a request sequence id; ignore out-of-order results) prevents flicker and races.
- **Panning past loaded edges** → `to` clamped to `now`; `from` clamped to the monitor's earliest data implicitly (empty results render nothing new). No infinite past.
- **Preserving the zoom view on data swap** → Use `update('none')` and avoid `resetZoom()` except on explicit reset, so refetch densifies without snapping the view.

## Open Questions

_None — refetch trigger, raw/bucketed selection heuristic, loop guarding, and reset behavior are decided above._

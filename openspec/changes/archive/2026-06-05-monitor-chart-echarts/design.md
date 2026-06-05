## Context

The detail page (`assets/monitor.html` + `monitor.js`) currently renders the response-time chart with Chart.js, the zoom plugin, and a date adapter, and refetches data on zoom for dynamic resolution. It consumes `GET /api/monitors/:name/series` (bucketed) and `/history` (raw), plus `/uptime`. The data model and endpoints are fine; only the rendering/interaction layer is being replaced. Hard-won constraints from the Chart.js saga: (a) panning needs a gesture lib (we never had one), (b) storing the chart instance on the Alpine component causes infinite recursion over the chart's circular refs, (c) destroying/recreating mid-interaction crashes. ECharts addresses (a) natively and we carry (b)/(c) forward as design rules.

## Goals / Non-Goals

**Goals:**
- Built-in, reliable pan + zoom (no plugin, no Hammer).
- Downtime clearly shown: line interrupted at outages + red vertical band over the down span.
- Keep dynamic resolution (refetch finer/coarser for the visible range) via ECharts' zoom event.
- No stack-overflow: chart instance + data kept out of Alpine reactivity.
- Dark theme consistent with the app.

**Non-Goals:**
- Backend/API/data-model changes (reuse `/series`, `/history`, `/uptime`).
- A bundler/build step — ECharts is loaded from a pinned CDN, assets stay embedded.
- Preserving the exact Chart.js look (gauges/metrics strip/window selector stay; only the chart engine changes).

## Decisions

### 1. ECharts via pinned CDN; instance + data in closures
Load `echarts@5.5.1` from jsDelivr (verified resolves). In `monitorDetail()`, keep `let chart = null;` and the data arrays (`points`, `downIntervals`, `pointMeta`) as **closure variables**, never properties of the returned reactive object. The template binds only primitives (`loading`, `activeWindow`, `sampleCount`, `lossCount`, `chartMode`, `isLoadingDetail`, `hasData`, `uptimeWindows`). This prevents Alpine from proxying ECharts' internals.

### 2. Chart container is a sized div
ECharts renders into a `<div>` with explicit dimensions (not `<canvas>`). Replace the `chart-canvas-wrap`/`<canvas>` with `<div id="response-chart">` sized via CSS (e.g. height 340px, width 100%). Call `chart.resize()` on window resize.

### 3. Series + downtime rendering
- One `line` series: `data` as `[ [tsMs, avg_ms], ... ]` on a `time` xAxis, `connectNulls: false`, smooth, thin cyan stroke (optional light area). Down points contribute a `null` y so the line **breaks** at outages.
- Downtime bands: compute `downIntervals` by merging consecutive down samples (raw: `status==='down'`; series: `up_ratio < 1`) into `[startMs, endMs]` spans, and render as `series.markArea` with a translucent red `itemStyle`. This yields the "vertical red bar"/band over each outage. (Single-sample outages get a thin band from the sample to the next sample/bucket edge.)
- `tooltip: { trigger: 'axis' }` showing time + response (and "down"/loss when applicable).

### 4. Pan/zoom via dataZoom
`dataZoom: [{ type: 'inside', xAxisIndex: 0 }, { type: 'slider', xAxisIndex: 0 }]`. `inside` gives wheel-zoom and drag-pan within the plot; `slider` adds a draggable range bar beneath. No Hammer, no plugin. A `minSpan` (or `minValueSpan`) caps zoom-in to a sane minimum (e.g. ~10s).

### 5. Dynamic resolution via the `datazoom` event
`chart.on('datazoom', ...)`: read the realized visible range from `chart.getModel()`/`getOption().dataZoom` `startValue`/`endValue` (epoch ms), debounce (~300ms), then refetch — `/history` if the range holds few enough samples to render raw, else `/series` — and `setOption` with the new line data + recomputed downtime bands. Guard re-entrancy with an `isApplying` flag and a request sequence id (drop stale responses); `setOption` merges without resetting the zoom (`notMerge: false`). Selecting a window sets the base range, loads it, and resets the zoom to full.

### 6. Window selector + reset
The uptime windows (1h/24h/7d/30d) remain clickable: each sets the active range, fetches its data, and `setOption`s with the zoom reset to the full window. A reset control calls `chart.dispatchAction({ type: 'dataZoom', start: 0, end: 100 })` and reloads the window.

## Risks / Trade-offs

- **Bundle size (~1MB ECharts)** vs Chart.js+plugins (~300KB). Acceptable for a self-hosted dashboard; could switch to a custom ECharts build later if needed.
- **Single-sample downtime bands** may be very thin at wide zoom; merging consecutive downs and using a minimum band width keeps them visible. Aggregated buckets with partial loss are shaded for the whole bucket (slightly over-represents), which is acceptable for visibility.
- **`datazoom` fires often** during drag; debounce + stale-response dropping prevents server spam and flicker (same approach as before, now on a clean event).
- **Browser-only verification** — interactions can't be tested headlessly; the change must be validated in a browser (pan, wheel zoom, slider, downtime bands, dynamic resolution, no console errors).
- **Supersedes pending Chart.js changes** — `chart-enable-pan-hammer` should be dropped; the Chart.js code from `monitor-chart-pan-and-fit` is replaced wholesale.

## Open Questions

_None — library (ECharts), interactions (dataZoom), downtime rendering (markArea + line gaps), and the non-reactive instance rule are decided._

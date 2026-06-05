## Why

The monitor detail chart has been a recurring source of breakage on Chart.js + `chartjs-plugin-zoom`: panning needs Hammer.js (never loaded), pinch needed Hammer too, the filler path crashed on zoom, and storing the chart on the Alpine component caused stack-overflows. Pan and reliable zoom-out still don't work. Rather than keep patching that stack, switch to **Apache ECharts**, which provides pan/zoom and downtime shading out of the box.

## What Changes

- **Replace Chart.js + chartjs-plugin-zoom + chartjs-adapter-date-fns (+ the never-loaded Hammer.js) with Apache ECharts** for the monitor detail response-time chart.
- **Pan/zoom built in**: ECharts `dataZoom` — wheel-zoom + drag-pan inside the plot, plus a draggable range slider — no plugin, no Hammer.
- **Downtime visualization**: where a probe is down (raw) or an interval has loss (aggregated), the response-time line is **interrupted (gap)** and the downtime span is shaded with a **red vertical band** (ECharts `markArea`), so outages are unmistakable on the chart.
- **Dynamic resolution preserved**: subscribe to the ECharts `datazoom` event to refetch finer/coarser data (raw vs aggregated `/series`) for the visible range, debounced — same behavior, via a clean event instead of fighting a plugin.
- **Keep the chart instance non-reactive**: hold the ECharts instance and chart data in `monitorDetail()` closure variables (never on the Alpine reactive object) so the library never recurses over reactive proxies (the prior stack-overflow cause).
- Pin the ECharts CDN version.

This supersedes the Chart.js-specific pending changes `chart-enable-pan-hammer` and the chart bits of `monitor-chart-pan-and-fit`; those should be abandoned. `probe-result-rollups` (data layer) is unaffected and still worthwhile.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `web-ui`: The monitor detail response-time chart is rendered with ECharts, providing built-in wheel-zoom/drag-pan and a range slider, dynamic-resolution refetch on zoom, and explicit downtime visualization (line interruption + red vertical band).

## Impact

- `assets/monitor.html` — remove the Chart.js/zoom-plugin/date-adapter `<script>` tags; add a pinned ECharts `<script>`; the chart container becomes a sized `<div>` (ECharts renders into a div, not a `<canvas>`).
- `assets/monitor.js` — rewrite the chart logic for ECharts: `echarts.init` (instance in closure), `setOption` with a time-axis line, `dataZoom` (inside + slider), `markArea` downtime bands, `datazoom`-event-driven debounced refetch (series/raw), window selector, dark theme. Remove Chart.js-era code (limits/fit/never-destroy workarounds).
- `assets/style.css` — chart container sizing for the ECharts div.
- No backend or API changes (still consumes `/series` and `/history`).

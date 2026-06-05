## 1. Swap Library (HTML + container)

- [x] 1.1 In `assets/monitor.html`, remove the Chart.js, `chartjs-plugin-zoom`, and `chartjs-adapter-date-fns` `<script>` tags (and any Hammer tag)
- [x] 1.2 Add a pinned ECharts script: `<script src="https://cdn.jsdelivr.net/npm/echarts@5.5.1/dist/echarts.min.js"></script>`
- [x] 1.3 Replace the `<canvas id="response-chart">` with a sized `<div id="response-chart">` (ECharts renders into a div); keep the chart card, window selector, reset control, and loading indicator
- [x] 1.4 In `assets/style.css`, size the chart `<div>` (e.g. height 340px, width 100%)

## 2. ECharts Rendering (monitor.js)

- [x] 2.1 Refactor `monitorDetail()` to hold the ECharts instance and chart data as closure variables (`let chart, points, downIntervals, pointMeta`) — never on the reactive returned object; expose only primitives (+ `hasData`) to the template
- [x] 2.2 Initialize with `echarts.init(el)` on a fresh load; build the option: `time` xAxis, value yAxis, one `line` series with `data` as `[tsMs, ms]`, `connectNulls: false`, dark-theme axis/label colors, `tooltip: { trigger: 'axis' }`
- [x] 2.3 Set null y for down samples so the line breaks at outages; compute `downIntervals` by merging consecutive down samples (raw: `status==='down'`; series: `up_ratio < 1`) and render them as `series.markArea` with a translucent red `itemStyle`
- [x] 2.4 Add `dataZoom: [{ type: 'inside', xAxisIndex: 0 }, { type: 'slider', xAxisIndex: 0 }]` with a sane `minValueSpan` (~10s); update the chart via `setOption` (merge) so zoom state is preserved on data refresh
- [x] 2.5 Call `chart.resize()` on `window` resize

## 3. Dynamic Resolution + Window Selector

- [x] 3.1 `chart.on('datazoom', ...)`: read the visible range (startValue/endValue, epoch ms), debounce (~300ms), and refetch — `/history` when the range holds ≤ the raw budget (via `/series` count), else `/series` buckets — then `setOption` with new line data + recomputed `downIntervals`; guard with `isApplying` + a request sequence id (drop stale responses)
- [x] 3.2 `selectWindow(label)` sets the active range, loads it, `setOption`s, and resets the zoom to full (`dispatchAction({ type: 'dataZoom', start: 0, end: 100 })`)
- [x] 3.3 `resetZoom()` resets the dataZoom to full and reloads the active window; keep the uptime windows clickable as the range selector
- [x] 3.4 Keep the metrics strip (`lastResponseMs`, `uptime24h`, last check, `sampleCount`) and `lossCount`/`chartMode` reactive state working with the new data flow

## 4. Verification

- [x] 4.1 `cargo build` (assets re-embed) and `node --check assets/monitor.js` pass; served `monitor.html` references ECharts (no Chart.js/zoom/adapter/Hammer tags)
- [ ] 4.2 (browser-only — needs your verification) Browser check (release build, 30-day data): the page loads with no console errors; wheel zoom, drag-pan, and the range slider all work; zooming changes resolution (raw on deep zoom, buckets when wide); downtime shows as a line gap + red band; reset returns to the window
- [x] 4.3 Update `README.md` if it names the charting library; note the monitor chart uses ECharts with built-in zoom/pan and downtime shading
- [x] 4.4 Abandon the now-superseded `chart-enable-pan-hammer` change (and the Chart.js bits of `monitor-chart-pan-and-fit`) so they aren't applied/archived

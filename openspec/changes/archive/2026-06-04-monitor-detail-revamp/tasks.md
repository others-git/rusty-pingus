## 1. Aggregated Series Query (DB)

- [x] 1.1 Add `db::get_series(pool, monitor_name, from, to, buckets) -> Result<Vec<SeriesBucket>>` in `src/db/mod.rs`: derive `bucket_secs = max(1, (to-from)/buckets)`, group by `CAST(strftime('%s', checked_at) AS INTEGER) / bucket_secs` over the `[from,to]` range
- [x] 1.2 Define `SeriesBucket { ts: String, avg_ms: Option<f64>, min_ms: Option<i64>, max_ms: Option<i64>, count: i64, up_ratio: f64 }` (Serialize); compute `ts` as bucket_index * bucket_secs in ISO-8601, `up_ratio` via `AVG(CASE WHEN status='up' THEN 1 ELSE 0 END)`
- [x] 1.3 Return an empty vec when no rows fall in the range

## 2. Series API Endpoint

- [x] 2.1 Add `SeriesParams { from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>, buckets: Option<i64> }` and a `series` handler in `src/api/mod.rs`: default `to=now`, `from=now-24h`, `buckets=300`; clamp `buckets` to 50..=1000
- [x] 2.2 Handler returns `Json(get_series(...))` (HTTP 200, empty array when no data); on DB error return 500
- [x] 2.3 Register `GET /api/monitors/:name/series` in `src/web/mod.rs` (alongside the existing `/history` and `/uptime` routes, before the catch-all)

## 3. Default Retention

- [x] 3.1 Add `DEFAULT_RETENTION_DAYS` (90) constant; in `src/main.rs` always spawn the retention loop using `cfg.defaults.retention_days.unwrap_or(DEFAULT_RETENTION_DAYS)`
- [x] 3.2 Update the `DEFAULT_CONFIG` comment in `src/config/mod.rs` to note the 90-day default when `retention_days` is unset

## 4. Monitor Page — Chart Dependencies & Time Scale

- [x] 4.1 In `assets/monitor.html`, add CDN `<script>` tags for `chartjs-adapter-date-fns` and `chartjs-plugin-zoom` (after the existing Chart.js script)
- [x] 4.2 In `monitor.js`, register the zoom plugin and switch the chart x-axis to a `time` scale; build datasets as `{ x: ts, y: avg_ms }` from the `/series` response
- [x] 4.3 Configure the zoom plugin: wheel + drag zoom and pan on the x-axis; keep the dark theme, gradient fill, and tooltip (show bucket avg + sample count)
- [x] 4.4 Indicate buckets with `up_ratio < 1` (e.g. a red point/marker) so downtime is visible on the chart

## 5. Monitor Page — Data-First Layout & Window Selector

- [x] 5.1 In `monitor.html`, reorder to data-first: header status, then a compact metrics strip (latest response time, 24h uptime, last check relative time, sample count), then the chart as the primary card, then the (now clickable) uptime windows, then a capped raw history table
- [x] 5.2 Make the uptime windows clickable controls; add `activeWindow` state and an `x-on:click` that sets the range and reloads the chart; highlight the active window
- [x] 5.3 Add `selectWindow(key)` in `monitor.js`: map 1h/24h/7d/30d to a `from` offset, fetch `/series?from=&to=now&buckets=300`, and re-render the chart
- [x] 5.4 Add a "Reset zoom" control that calls the chart's `resetZoom()` to return to the active window
- [x] 5.5 Populate the metrics strip from the latest history row + `/uptime`; cap the raw history table rows (e.g. most recent 100)
- [x] 5.6 Add any needed CSS to `assets/style.css` (e.g. active-window styling, chart sizing) consistent with the existing dark theme

## 6. Verification

- [x] 6.1 `cargo build` and `cargo clippy -- -D warnings` pass
- [x] 6.2 `cargo test` passes (add a `get_series` test: insert results across a span, assert bucket count ≤ target and up_ratio correctness)
- [x] 6.3 Smoke test series endpoint: seed results, call `/api/monitors/:name/series` with a wide range + small `buckets`, confirm bounded bucket count and correct avg/up_ratio; confirm empty range returns `[]`
- [x] 6.4 Smoke test UI: open a monitor page, confirm data-first layout, wheel/drag zoom + pan work, reset zoom returns to window, and clicking 1h/24h/7d/30d reloads the chart range
- [x] 6.5 Smoke test retention default: with no `retention_days` configured, confirm the retention loop runs with the 90-day default (e.g. via startup log / behavior)
- [x] 6.6 Update `README.md`: document the `/series` endpoint and the default 90-day retention behavior

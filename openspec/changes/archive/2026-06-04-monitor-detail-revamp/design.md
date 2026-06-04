## Context

The monitor detail page (`assets/monitor.html` + `monitor.js`) renders three stacked cards: SVG uptime rings (1h/24h/7d/30d from `/uptime`), a Chart.js line chart of the last 100 raw probes (`/history?limit=100`) on a category x-axis labelled with time strings, and a raw history table. `GET /api/monitors/:name/history` returns up to 1000 raw rows for a `[from,to]` range; `get_uptime` computes an up-ratio over a window. `probe_results` is indexed on `(monitor_name, checked_at DESC)`. Retention pruning exists (`prune_old_results`) but only runs when `defaults.retention_days` is set, so the default is unbounded growth — acute at sub-second poll intervals.

## Goals / Non-Goals

**Goals:**
- A data-first detail layout: status + metrics first, chart primary, raw table secondary.
- A response-time chart on a true time scale that supports wheel/drag zoom and pan, with reset.
- Uptime windows that, when clicked, set the chart's active range and load matching data.
- A bucketed series endpoint returning a bounded number of points for any window.
- A default retention so the DB is pruned by age out of the box.

**Non-Goals:**
- Background rollup/aggregate tables or tiered retention (query-time aggregation is sufficient here).
- Changing how raw probe results are written or the probe cadence.
- Realtime/streaming chart updates (the page stays poll/load based).
- Per-bucket percentiles beyond min/max/avg (p95 etc. left for later).

## Decisions

### 1. New `GET /api/monitors/:name/series?from=&to=&buckets=N`
Returns an array of buckets, each `{ ts, avg_ms, min_ms, max_ms, count, up_ratio }`, where `ts` is the bucket start (ISO-8601). The server derives `bucket_secs = max(1, (to-from)/buckets)` and groups in SQL. This bounds both payload size and chart point count regardless of window or poll interval. Defaults: `to=now`, `from=now-24h`, `buckets≈300` (capped, e.g. 50–1000). Alternative — keep returning raw rows and thin client-side — rejected: still ships huge payloads for long windows.

### 2. SQLite time bucketing
`probe_results.checked_at` is ISO-8601 text, so `CAST(strftime('%s', checked_at) AS INTEGER)` yields epoch seconds. Bucket index = `epoch / bucket_secs`; group by it and aggregate: `AVG/MIN/MAX(response_time_ms)`, `COUNT(*)`, `AVG(CASE WHEN status='up' THEN 1 ELSE 0 END)` as up-ratio. Bucket start `ts = bucket_index * bucket_secs`. The existing `(monitor_name, checked_at)` index serves the range scan. `down` probes have `NULL` response_time_ms and are naturally excluded from response aggregates while still counting toward `up_ratio`.

### 3. Chart on a time scale with `chartjs-plugin-zoom`
Switch the x-axis to Chart.js `time` scale (via `chartjs-adapter-date-fns`, loaded from CDN). Data points become `{ x: ts, y: avg_ms }`. Register `chartjs-plugin-zoom` for wheel + drag zoom and pan on the x-axis; expose a "Reset zoom" control. Down buckets are indicated (e.g. a red marker/band) using `up_ratio < 1`. Alternative — custom zoom on the category axis — rejected as more code and less natural panning.

### 4. Clickable uptime windows drive the active range
The uptime ring components become buttons. Selecting a window sets `activeWindow` (1h/24h/7d/30d → a `from` offset), refetches `/series` for `[now-window, now]` with the standard bucket count, and re-renders the chart; the selected window is highlighted. Zoom/pan then explores within the loaded range; "reset zoom" restores the full active window. The uptime *percentages* still come from `/uptime`.

### 5. Data-first layout
Reorder `monitor.html`: (a) header status (kept), (b) a compact metrics strip — last response time, 24h uptime, last check (relative), total checks in window, current up/down — (c) the chart with the window selector + reset control as the dominant card, (d) uptime rings (now the clickable selectors, more compact), (e) raw history table collapsed/secondary with a row cap. Keep the existing dark theme and Tailwind utility style.

### 6. Default retention
When `defaults.retention_days` is unset, apply a default (90 days) rather than retaining forever: the retention loop always runs with `retention_days = configured.unwrap_or(DEFAULT_RETENTION_DAYS)`. This caps growth without operator action. Operators can still set an explicit value (including a large one) to override. This is a behavioural change and is documented in the README and config comments.

## Risks / Trade-offs

- **Default retention deletes old data for users who relied on indefinite history** → Document clearly in README and the generated `config.toml` comment; the value is overridable. 90 days is a conservative, common default.
- **Bucketing accuracy at boundaries** → Buckets are aligned to epoch/`bucket_secs`, so the first/last bucket may be partial. Acceptable for a trend chart; tooltip shows bucket start + count.
- **Time-zone/label rendering** → date adapter renders in the browser's locale/timezone, consistent with the existing `toLocaleTimeString` behaviour.
- **Extra CDN dependencies** → `chartjs-plugin-zoom` + `chartjs-adapter-date-fns` add two `<script>` tags; acceptable since Chart.js is already CDN-loaded and assets are embedded at build time.
- **Very large `buckets` request** → Clamp server-side (e.g. 50–1000) to bound work and payload.

## Migration Plan

1. Add the `series` endpoint + `get_series` query (additive; `/history` and `/uptime` unchanged).
2. Update the monitor page to consume `/series`, add zoom/pan, and wire clickable windows.
3. Switch retention to always-on with the default; on first run after upgrade, results older than the default are pruned on the next retention tick. Operators who want the old behaviour set a large `retention_days`.
4. Rollback: revert the binary; the new endpoint simply disappears and the page falls back with a redeploy of old assets. No schema changes to undo.

## Open Questions

_None — aggregation approach, chart libraries, window behaviour, and retention default are decided above._

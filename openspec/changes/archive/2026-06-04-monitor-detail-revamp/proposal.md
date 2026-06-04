## Why

The monitor detail page leads with decorative uptime rings and a fixed "last 24h" chart of the most recent 100 raw probes. It isn't data-first, the chart can't be zoomed or panned to explore a time range, and the uptime windows are inert. Worse, the page (and the underlying `/history` query) reads raw `probe_results` rows — with low poll intervals (e.g. 500 ms) a single monitor produces ~170k rows/day, so long windows return huge payloads and the table/chart degrade, while the database grows without bound.

## What Changes

- **Data-first monitor detail layout**: lead with the live status and a compact metrics strip (current/last response time, 24h uptime, last check, total checks), make the response-time chart the primary element, and demote the raw history table.
- **Zoomable / pannable chart**: switch the response-time chart to a real time-scale x-axis and add wheel/drag zoom and pan (via `chartjs-plugin-zoom` + a date adapter, loaded from CDN like Chart.js already is). A "reset zoom" affordance returns to the active window.
- **Clickable uptime windows set the time scale**: clicking a window (1h / 24h / 7d / 30d) sets the chart's active range to that window and loads the corresponding data. The selected window is visually indicated.
- **Aggregated time-series endpoint** (`GET /api/monitors/:name/series`): buckets probe results server-side into a bounded number of points per window (avg / min / max response time and up-ratio per bucket), so any window — including 30d at sub-second intervals — returns a small, fast payload. The chart consumes this instead of raw rows.
- **Bounded history growth**: apply a sensible **default retention** so `probe_results` is pruned by age even when the operator hasn't configured one, capping unbounded growth. **BREAKING** for anyone relying on indefinite retention by default.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `web-ui`: The monitor detail page becomes data-first with a metrics strip, a zoomable/pannable time-scale response-time chart, and clickable uptime windows that drive the chart range; a new `GET /api/monitors/:name/series` JSON endpoint returns bucketed response-time/up-ratio data.
- `result-storage`: Add an aggregated probe-series query that buckets results over a time range into a bounded number of points; change the default retention policy so results are pruned by age by default.

## Impact

- `assets/monitor.html`, `assets/monitor.js`, `assets/style.css` — data-first layout, time-scale chart, zoom/pan, clickable windows, CDN script tags for the zoom plugin + date adapter.
- `src/db/mod.rs` — new bucketed aggregation query (`get_series`); default retention constant.
- `src/api/mod.rs`, `src/web/mod.rs` — new `series` handler + route.
- `src/main.rs` / `src/config/mod.rs` — apply the default retention when none is configured.
- `README.md` — document the series endpoint and the default retention behavior.

## Why

The monitor detail chart loads a fixed ~300 aggregated buckets for the selected window. The zoom/pan controls magnify those same points, so zooming in does not reveal any additional detail — the resolution is fixed at load time. Users expect that zooming in far enough surfaces finer data (and ultimately individual probes), the way mapping and metrics tools behave.

## What Changes

- **Progressive (level-of-detail) chart loading**: when the user zooms or pans, the chart refetches data for the *currently visible* time range, so the visible window is always rendered at ~300 points of resolution rather than the original window's resolution.
- **Raw probes at deep zoom**: when the visible range is small enough that every individual probe fits within the point budget, the chart loads raw probe points (from `/history`) instead of aggregated buckets — so zooming in far enough shows each probe.
- **Debounced refetch + loading affordance**: zoom/pan refetches are debounced to avoid spamming the server, with a subtle "updating" indicator while finer data loads.
- **Reset returns to the active window**: the existing reset control restores the active uptime window and its resolution.
- No backend changes: this reuses the existing `GET /api/monitors/:name/series` (arbitrary `from`/`to`/`buckets`) and `GET /api/monitors/:name/history` (arbitrary `from`/`to`/`limit`).

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `web-ui`: The monitor detail chart gains dynamic, level-of-detail loading — it refetches finer data for the visible range on zoom/pan and switches from aggregated buckets to raw individual probes when zoomed in far enough.

## Impact

- `assets/monitor.js` — zoom/pan completion handlers, debounced visible-range refetch, raw-vs-bucketed source selection, loading flag, reset behavior.
- `assets/monitor.html` — a small "updating" indicator on the chart card.
- Depends on `monitor-detail-revamp` (the `/series` endpoint, time-scale chart, and zoom plugin); that change should be archived first so the main `web-ui` spec reflects the revamped detail page.
- No backend or schema changes.

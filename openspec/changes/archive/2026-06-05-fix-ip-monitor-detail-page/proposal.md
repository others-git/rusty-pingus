# Fix the public-IP monitor detail page

## Why

The public-IP ("check IP") detail page shows an empty graph and "Current IP: no data" even though the dashboard shows the IP correctly. Root cause: the detail page builds its chart time window from the **browser's clock** (`[Date.now() − window, Date.now()]`) and sends those absolute bounds to the history/series API, while probe timestamps are recorded on the **server's clock**. When the two clocks differ (observed here: server ~4h40m ahead of the browser, a routine WSL2 host/guest drift), every probe falls outside the requested window and the ranged query returns nothing. The dashboard and the page header are unaffected because they use the latest-probe snapshot, which has no time window.

A second problem compounds it: the recently added per-probe auto-refresh re-runs that same (skewed, empty) query every second and fully re-renders the chart, which **flashes** and provides no value for a public-IP monitor whose address rarely changes. Public-IP monitors also surface response-time framing that is irrelevant to their purpose.

## What Changes

- **Fix the empty graph (clock-skew robustness):** Anchor the detail page's time windows to the most recent **server-recorded** probe timestamp rather than the browser clock, so the chart/timeline and metrics remain correct regardless of client/server clock skew. This fixes every monitor type's detail chart, not just public-IP.
- **Stop the flashing / unnecessary refresh on public-IP:** Public-IP (and other state-timeline) detail pages SHALL NOT auto-reload the chart on every probe. The header's current value still updates live; the chart/timeline reloads only on user action (window select, zoom/pan, manual refresh) or when the tracked value actually changes — no per-probe re-render, no flashing.
- **Make the public-IP page purpose-built:** Remove response-time framing from the public-IP detail view; it tracks the external IP, uptime, and IP changes — not latency.
- Keep the existing "no data" vs "unreachable" distinction, which will now correctly show real data once the window is anchored properly.

## Capabilities

### Modified Capabilities
- **web-ui** — Detail page time windows are anchored to server-recorded probe time (skew-safe); the public-IP detail view is purpose-built (no response-time metrics/labels).
- **live-status-updates** — Refine detail-page live updates: state-timeline monitors (public-IP/border) update their header value live but do not auto-reload the chart per probe (no flashing); the chart refreshes on user action or an actual value change.

### New Capabilities
- None.

## Impact

- **Frontend:** `assets/monitor.js` (anchor `windowFrom`/`windowTo` and the live-slide to the latest server probe timestamp instead of `Date.now()`; suppress per-probe chart reload for timeline kinds; refresh on value change only); `assets/monitor.html` (ensure no response-time labels render for public-IP).
- **Backend:** None required — the latest probe timestamp is already available via the existing history/snapshot responses.
- **Specs:** `web-ui`, `live-status-updates`.
- **No API, schema, or dependency changes.**
- Partially revises the just-applied `live-detail-page` change (keeps live header updates; removes the per-probe chart reload that caused flashing for public-IP).

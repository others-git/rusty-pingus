# Live, self-healing monitor detail page

## Why

The monitor detail page (`/monitors/:name`) fetches once on load and never refreshes — this is mandated today by the *"Detail pages do not auto-update"* requirement. As a result, a page opened before data exists in its window (a newly added or renamed monitor, or just after a server restart) renders an empty/`unreachable` snapshot and stays frozen there, even though the dashboard — which is SSE-driven — shows the monitor as healthy. The detail page contradicting the dashboard for the same monitor reads as a bug and undermines trust in the page.

## What Changes

- **BREAKING (spec):** Reverse the *"Detail pages do not auto-update"* rule. The monitor detail page SHALL subscribe to the existing `GET /api/monitors/stream` SSE feed and refresh in place as new probe results arrive for the monitor it is showing, with a periodic poll as fallback (mirroring the dashboard).
- On each live update for the current monitor, refresh the header status/detail and the metrics strip, and refresh the active-window chart/timeline data so a monitor that had no in-window data fills in without a manual reload.
- Live refresh SHALL NOT disrupt an in-progress user interaction: it must not reset the user's current zoom/pan or fight a pending zoom-triggered refetch.
- Distinguish **"no data yet"** from **"down/unreachable"** in the detail view's empty states. When a window genuinely has no samples, the public-IP strip and chart SHALL show a *collecting/no-data-yet* state rather than `unreachable` (which should mean the latest probe actually failed).
- Provide a graceful fallback: when SSE is unavailable, the page SHALL keep itself current via a periodic poll.

## Capabilities

### Modified Capabilities
- **live-status-updates** — Replace the requirement that detail pages stay static with one that has detail pages subscribe to the live stream (with poll fallback) and refresh in place, scoped to the monitor being viewed.
- **web-ui** — Extend the *Monitor detail page* behavior to refresh live as updates arrive without disrupting zoom/pan, and to render a distinct "no data yet / collecting" empty state separate from "down/unreachable".

### New Capabilities
- None.

## Impact

- **Frontend:** `assets/monitor.js` (add SSE subscription + poll, in-place refresh of header/metrics/active-window data, empty-state handling that doesn't clobber zoom/pan); `assets/monitor.html` (empty-state copy for the public-IP strip and chart area).
- **Backend:** None expected — `GET /api/monitors/stream` and its `StatusUpdate` payload (`name`, `status`, `response_time_ms`, `last_checked_at`, `failure_reason`, `detail`) already exist and are sufficient.
- **Specs:** `live-status-updates`, `web-ui`.
- **No API, schema, or dependency changes.**

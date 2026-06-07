## Why

Retention is currently a single global `retention_days` (default 90) applied to every monitor — there's no way to keep, say, a noisy public-IP check for only a few hours while keeping a critical HTTP monitor for months. Separately, the detail page's state-timeline (public-IP/border) fetches only the newest 1000 raw rows in the window, so a fast-probing monitor appears to "only keep ~15 minutes" even though the data is in the database. And the fixed 1h/24h/7d/30d window selectors don't reflect how much data a monitor actually retains.

## What Changes

- **Per-monitor retention, in hours.** Every monitor gains a `retention_hours` setting (replacing reliance on the single global value for pruning granularity; the global default still applies when a monitor doesn't specify one). A periodic prune deletes each monitor's `probe_results` and rollups older than its own retention. The traceroute hop tables also prune by the same per-monitor retention (its legacy `retention_ms` is accepted as input and converted).
- **Render the full selected window on state-timeline detail pages.** Fix the public-IP/border timeline so it reflects the entire selected range instead of only the newest ~1000 raw probes — collapsing consecutive same-state probes server-side so the whole window is shown regardless of probe frequency.
- **Replace the fixed window buttons with a retention-bounded brush.** The detail page's time-range control becomes a resizable brush spanning the monitor's retained data (consistent with the traceroute detail page), instead of the fixed 1h/24h/7d/30d buttons. Uptime percentages remain shown as read-only stats.
- **Surface retention on the monitor page** and in the add-monitor form (collected in hours).

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- **monitor-config** — every monitor carries a `retention_hours` setting (default from the global retention; back-compatible when omitted); traceroute's `retention_ms` becomes a legacy alias.
- **result-storage** — retention is applied per monitor (each monitor's results/rollups pruned by its own retention) rather than one global value; traceroute hop pruning uses the same per-monitor retention.
- **web-ui** — the detail page replaces the fixed 1h/24h/7d/30d chart-range buttons with a retention-bounded time brush, renders the full selected window for state timelines (no newest-N cap), and surfaces/collects retention.

## Impact

- **Backend:** `monitors/mod.rs` (`retention_hours` on every variant + accessor; traceroute `retention_ms`→hours), `db/mod.rs` (per-monitor prune of `probe_results`/rollups by retention; a state-segment query that collapses same-state runs over a range so the timeline isn't capped), `scheduler/mod.rs` + `main.rs` (retention loop iterates monitors, pruning each by its own retention), `api/mod.rs` (`retention_hours` in list + accept on create).
- **Frontend:** `assets/monitor.js` (generalize the traceroute brush to the response-time + state-timeline pages; drive the chart range from a brush over `[now − retention, now]`; consume the new state-segment data so the full window renders), `assets/index.html`/`monitor.html` (retention field in the add form; show retention on the detail page).
- **Config:** `monitors.toml` entries may set `retention_hours`; existing files without it fall back to the global default. No DB schema change (pruning + a new read query only).
- **Specs:** `monitor-config`, `result-storage`, `web-ui`.

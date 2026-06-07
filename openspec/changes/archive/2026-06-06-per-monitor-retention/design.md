## Context

Retention today is one global `retention_days` (default 90) read in `main.rs` and applied by `scheduler::retention_loop` → `db::prune_old_results`/`prune_old_rollups` (cutoff = now − days, across all monitors). Traceroute already has its own per-monitor `retention_ms` pruning its hop tables on a 60 s loop.

The detail page (`assets/monitor.js`) loads state timelines (public-IP/border) via `history?from&to&limit=1000` and builds segments client-side by collapsing consecutive same-state rows. The `ORDER BY checked_at DESC LIMIT 1000` returns only the newest 1000 rows in the window, so a fast monitor shows only its most recent slice. The response-time chart uses the bucketed `series` endpoint (already whole-window). The fixed 1h/24h/7d/30d uptime selectors double as the chart-range control. The traceroute detail page already has a working retention-bounded brush we can generalize.

## Goals / Non-Goals

**Goals:**
- Per-monitor retention in hours governs how long that monitor's data is kept.
- The state-timeline detail page reflects the whole selected window regardless of probe frequency.
- The detail chart range is chosen with a retention-bounded brush, replacing the fixed window buttons.
- Backward compatible: monitors/config without the new field behave as today (global default).

**Non-Goals:**
- Changing the rollup algorithm or the response-time `series` path (already whole-window).
- A global "retention for all" bulk control.
- Removing uptime percentages (they stay as read-only stats).

## Decisions

### Decision: `retention_hours` on every monitor; global default as fallback; traceroute `retention_ms` becomes a legacy alias
Add `retention_hours: u64` to each monitor config (and `Raw*`), defaulting to the global retention when omitted (so existing files are unchanged in behavior). For traceroute, accept the legacy `retention_ms` as input and convert to hours; the unified `retention_hours` then governs both its `probe_results`/rollup pruning and its hop-table pruning, so there's one retention concept per monitor.
- *Why:* One per-monitor knob, expressed in the unit the user asked for, with no breakage for existing configs or the existing traceroute field.
- *Trade-off:* Hours is coarse for sub-hour retention (the public-IP "~15 min" case wants <1h). Mitigation: allow a minimum of 1 hour for `probe_results` retention, and note that the *display* fix (below) is what actually addresses the "15 min" view — retention just bounds storage. (If sub-hour retention is later wanted, the field can move to minutes.)
- *Alternative:* Keep traceroute's `retention_ms` separate from a new `retention_hours` — rejected; two retention knobs on one monitor is confusing.

### Decision: Per-monitor pruning loop
Replace the single global prune with a loop over the current monitors, pruning each monitor's `probe_results` + rollups older than its `retention_hours` (and traceroute hops by the same). Reuse the existing `traceroute_retention_loop` cadence/structure (read the live monitor list so changes apply without restart). `prune_old_results`/`prune_old_rollups` gain a `monitor_name` + ms-cutoff variant.
- *Why:* Different monitors need different cutoffs; the list-driven loop already exists for traceroute.
- *Trade-off:* Pruning is now N small deletes instead of one; cheap given indexes and a periodic (not hot-path) cadence.

### Decision: Server-side state segments for the timeline (no newest-N cap)
Add a query that returns collapsed state segments (`{ state, start, end }`) for a monitor over `[from, to]`, computed by scanning ascending and merging consecutive same-state probes — so the response is bounded by the number of *changes*, not the number of probes. The public-IP/border detail page consumes this instead of capping at 1000 raw rows.
- *Why:* A state timeline only needs transitions; returning the whole window's transitions is small even for high-frequency monitors, fixing the "~15 min" view.
- *Trade-off:* New read query + a frontend path change. The existing client-side `buildSegments` logic moves server-side (or the client keeps building from a complete-but-collapsed row set). Keeping per-probe `detail` (IP/fault class) is required for the state value.
- *Alternative:* Just raise `RAW_LIMIT` — rejected; unbounded for fast monitors.

### Decision: Retention-bounded brush replaces the fixed window buttons
Generalize the traceroute detail brush to the response-time and state-timeline pages: the brush spans `[now − retention, now]`; moving/resizing it sets the `[from, to]` passed to `series`/timeline/segment queries (debounced), exactly as traceroute does today. The 1h/24h/7d/30d uptime figures remain as read-only stat tiles (no longer chart-range buttons).
- *Why:* The user chose a unified brush; we already have a proven implementation to lift. Bounding the brush to retention means you can't scrub to a range with no data.
- *Trade-off:* Removes one-click "jump to 7d"; acceptable per the chosen direction. The brush starts showing the full retained window by default.

## Risks / Trade-offs

- **Hours floor vs the "15 min" symptom** → Retention in hours can't express 15 min, but the *display* fix is what makes the graph show the whole window; retention only bounds storage. Documented so the user isn't surprised.
- **Behavior change for existing configs** → Defaulting `retention_hours` to the global retention keeps current behavior; only explicit settings shorten data. Verify an omitted field prunes at 90 days as before.
- **Generalizing the brush to two more page types** → The response-time page currently relies on `selectWindow`/uptime buttons; replacing that control risks regressions in zoom/pan and live-refresh. Mitigation: reuse the traceroute brush + the existing server-anchored windowing; verify zoom/pan and live refresh still work.
- **Moving segment-building server-side** → Must preserve the exact state semantics (`extractState` leading token) and the "last segment extends to window end" behavior.

## Migration Plan

Config-field + frontend + pruning change; no DB schema migration (a new read query + per-monitor deletes only). Existing `monitors.toml` without `retention_hours` keeps the 90-day default. Traceroute `retention_ms` is read and converted. Ships with embedded assets; release rebuild to re-embed.

## Open Questions

- Keep the uptime % tiles purely informational, or drop them entirely once the brush is the range control? Proposed: keep as read-only stats.
- Default `retention_hours` — mirror the global 90 days (2160 h) for back-compat, or pick a friendlier default for new monitors? Proposed: inherit the global default (2160 h) unless the user sets one.
- Should sub-hour retention ever be needed (true 15-min keep)? Deferred; hours now, revisit if requested.

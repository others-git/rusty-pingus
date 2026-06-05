## Context

`assets/monitor.js` (ECharts) already merges consecutive down samples into `downIntervals = [[startMs, endMs], ...]` via `computeDownIntervals`, and renders them as a single `markArea` (`silent: true`, translucent red). The band width tracks outage duration, so short outages are sub-pixel at many zooms and the band carries no hover info. The line itself already breaks at outages (null y, `connectNulls: false`).

## Goals / Non-Goals

**Goals:**
- A downtime marker is visible at any zoom level (fixed-pixel element).
- Brief drops and sustained outages are visually distinct (tick vs fat labeled band).
- Hovering a marker shows its time (single timestamp, or start→end + duration).

**Non-Goals:**
- Changing how downtime is detected (still `status==='down'` raw / `up_ratio < 1` series).
- Backend/data changes; this is purely chart rendering.
- A separate incidents list/panel (could be a later change).

## Decisions

### 1. Classify each merged interval: brief vs sustained
For each `[start, end]` in `downIntervals`, treat it as **sustained** when its duration `end - start >= SUSTAINED_MS` (a tunable threshold, default ~60s), else **brief**. Rationale: duration is resolution-independent, unlike "consecutive sample count" (which means 500ms in raw vs 1min in series). Threshold is a single constant so it's easy to tune after seeing real data.

### 2. Brief drop → always-visible vertical tick (`markLine`)
Render brief intervals as ECharts `markLine` verticals at the interval start (`data: [{ xAxis: start }]`), thin red, fixed pixel width → never sub-pixel. This is the zoom-stable guarantee. Give *every* interval (brief and sustained) a markLine so even a long outage stays locatable when zoomed so far out that its band is sub-pixel.

### 3. Sustained outage → fat labeled band (`markArea`)
Keep `markArea` for sustained intervals, with a `label` showing start time + duration (e.g. "14:03 · 1m40s"). Label placement inside/above the band. Brief intervals get no band (just the tick), so the chart isn't littered with micro-bands.

### 4. Hover reveals timestamps (markers not `silent`)
Set the markers interactive (`silent: false`) and surface the time on hover:
- `markLine`: `emphasis`/`label` (or tooltip) showing the drop's timestamp.
- `markArea`: a `name`/`label` showing `start → end (duration)`.
Confirm exact ECharts hover/tooltip wiring during implementation (markLine label-on-emphasis is reliable; markArea may need a label rather than a tooltip).

### 5. Keep the line gap
The response line continues to break at outages (unchanged); the tick/band augment it.

## Risks / Trade-offs

- **Marker density at deep zoom with noisy data** — the synthetic backfill has ~0.3% random loss, so a raw view could show many ticks. Real ICMP loss is burstier. If it's noisy, options: only mark intervals with ≥2 consecutive drops, or only tick sustained outages. Leave the threshold/filter tunable; revisit after seeing real data.
- **`SUSTAINED_MS` choice** — 60s is a guess; a 2s blip vs a 5-min outage is clearly brief vs sustained, but the exact cutoff is cosmetic and tunable.
- **ECharts markArea hover support is limited** — if a band tooltip proves awkward, fall back to a label on the band (always shown) plus the markLine tooltip for the precise timestamp.
- **Browser-only verification** — visual/hover behavior can't be tested headlessly; verify by zooming onto a brief drop and a sustained outage.

## Open Questions

- Exact `SUSTAINED_MS` and whether to suppress single-sample ticks entirely if deep-zoom noise is bad (decide after seeing real loss patterns).

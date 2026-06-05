## Why

The monitor chart shows downtime as an ECharts `markArea` band whose width is proportional to outage-duration ÷ visible-span. That makes short outages (a single dropped probe ≈ one poll interval) sub-pixel and invisible at many zoom levels — they "disappear" as you zoom in (the series→raw transition collapses a fat minute-band to a ~500ms sliver). The bands are also `silent`, so there's no way to see *when* an outage happened on hover.

## What Changes

- **Every outage gets a zoom-stable marker**: in addition to (or instead of) the duration band, render a thin vertical red line (`markLine`) at each outage so it stays visible at any zoom level — a dropped packet never vanishes.
- **Sustained outages are fat and labeled**: an outage that lasts beyond a short threshold is drawn as a filled red band (`markArea`) with a label showing its start time and duration; brief blips stay as a thin tick.
- **Hover shows the timestamp**: hovering a downtime marker reveals its time — a single timestamp for a brief drop, and the start→end range (and duration) for a sustained outage. (Markers are no longer `silent`.)

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `web-ui`: Downtime markers on the monitor chart stay visible at any zoom, distinguish brief drops (thin tick) from sustained outages (fat, labeled band), and reveal their timestamp/duration on hover.

## Impact

- `assets/monitor.js` — classify merged down-intervals as brief vs sustained (by duration / consecutive-down span); render brief ones as a `markLine` (always-visible vertical line) and sustained ones as a labeled `markArea` band; add hover labels/tooltip with timestamps; remove `silent: true`.
- Depends on `monitor-chart-echarts` (the ECharts chart + `computeDownIntervals`); that change should be archived/applied first.
- No backend or API changes.

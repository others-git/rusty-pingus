## Why

A recent fix added an explicit x-axis `min`/`max` set to the full selected window plus a zoom-plugin `limits` block. Two regressions resulted: (1) on load the data is bunched against the right edge instead of filling the chart, because the axis is forced to span the whole window even when the loaded data only covers part of it; and (2) panning does nothing, because the view already equals the `limits` extent, leaving no room to pan. The chart needs to open fitted to its data and pan freely.

## What Changes

- **Fit the view to loaded data on load**: when a window is selected (or first loaded), set the x-axis range to the actual extent of the loaded data so points fill the chart, rather than forcing the axis to the full requested window.
- **Unconstrain pan/zoom**: remove the zoom-plugin `limits` block that pinned the view to the window and prevented panning. Pan and zoom operate freely; the existing debounced refetch loads data for whatever range becomes visible (older history via pan, finer/coarser via zoom).
- **Do not override the view during interaction**: explicit axis bounds are set only on a fresh window load, never on a zoom/pan-triggered refetch, so the user's gesture position is preserved while data densifies/coarsens.
- **Pin chart libraries**: pin `chart.js`, `chartjs-plugin-zoom`, and `chartjs-adapter-date-fns` to specific known-good versions instead of floating major-version CDN tags, to stop version drift from reintroducing chart/zoom breakage.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `web-ui`: The monitor detail chart opens fitted to its loaded data (no right-edge bunching) and supports free panning/zooming; the dynamic-resolution refetch no longer overrides the view during a gesture.

## Impact

- `assets/monitor.js` — remove the zoom `limits` block and the forced window `min`/`max`; set x-axis min/max to the loaded data extent on a fresh window load only; keep never-destroy, refetch-on-view-change, and series/raw routing.
- `assets/monitor.html` — pin the CDN `<script>` tags for Chart.js, the zoom plugin, and the date adapter to exact versions.
- No backend or API changes.

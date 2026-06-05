## Context

The detail chart (`assets/monitor.js`) keeps one persistent Chart.js instance and refetches data for the visible range on zoom/pan (dynamic resolution). A prior fix, intended to enable zoom-out coarsening, set `scales.x.min/max` to the selected window `[now-window, now]` and added a zoom-plugin `limits.x` pinned to that same window. Consequences:
- The axis always spans the full window. When loaded data covers only part of it (e.g. the app probed for a few hours within a 24h window, or `now` is later than the last probe), points compress against the right edge.
- `limits.x.min/max == window == current view`, so the pan plugin has no slack to move → panning is a no-op.

Chart.js is loaded from a floating `chart.js@4` CDN tag (currently 4.5.1) with `chartjs-plugin-zoom@2` and `chartjs-adapter-date-fns@3`; recent breakage (filler crash, pinch/Hammer crash) tracks with this surface, so version drift is a standing risk.

## Goals / Non-Goals

**Goals:**
- Chart opens with data filling the plot area (fitted to the loaded data extent).
- Panning works (drag to move through time); zoom in/out works and refetches resolution.
- Gesture position is preserved across the densify/coarsen refetch.
- Stabilize the chart dependency versions.

**Non-Goals:**
- Re-introducing the gradient fill (kept off due to the filler/zoom crash).
- Changing the rollup/series/raw data model or any backend.
- Touch pinch-zoom (still needs Hammer.js, intentionally not loaded).

## Decisions

### 1. Fit the axis to loaded data on a fresh load; don't force the window
On a window select (and the initial load), after data is fetched, set `scales.x.min`/`max` to the first and last loaded point timestamps (or leave them unset and let Chart.js auto-fit). This fills the plot area regardless of where the data sits within the requested window. The requested window still determines *what data is fetched*; it no longer dictates the axis bounds.

### 2. Remove the zoom `limits` block
Without `limits`, the pan plugin is not clamped, so dragging moves the view freely — including past the loaded data into older history, which the existing `onPanComplete` refetch then loads. This is the fix for "panning does nothing." Zoom-out is likewise unclamped; `onZoomComplete` refetches the widened range at coarser resolution.

### 3. Set explicit bounds only on fresh loads, never during gestures
`renderChart` (called by both fresh selects and zoom/pan refetches) must not reset `scales.x.min/max` on refetch — only the data is swapped (`update('none')`). Explicit min/max are applied solely in the window-select path, so a zoom/pan gesture's position is preserved while data densifies or coarsens. This keeps the never-destroy, single-instance model from the previous fix.

### 4. `resetZoom` re-fits to the active window
Reset re-runs the window select, which refetches the window and re-fits the axis to that data — returning to the overview without relying on the plugin's cached original limits.

### 5. Pin chart dependencies
Pin Chart.js, `chartjs-plugin-zoom`, and `chartjs-adapter-date-fns` to exact, mutually-tested versions in the asset `<script>` tags. This removes CDN major-tag drift as a source of regressions. The pinned set is verified together in the browser as part of this change.

## Risks / Trade-offs

- **Free pan/zoom into empty regions** (before first data or after the last probe) → the chart shows empty space there and refetch returns nothing; acceptable and normal for a pannable time chart.
- **Auto-fit vs explicit data-extent bounds** → if auto-fit proves to fight the zoom plugin's state, fall back to explicitly setting min/max to the data extent on fresh loads; both achieve "filled on load".
- **Browser-only behavior** → zoom/pan/centering cannot be verified headlessly; this change MUST be validated interactively in a browser (see tasks), ideally against the large 30-day dataset.
- **Pinning may pin a bug** → choose versions known to work with this combination; document them so future bumps are deliberate.

## Open Questions

- Exact pinned versions: pick the latest set verified working together in-browser during implementation (e.g. Chart.js 4.4.x with zoom 2.0.x + adapter 3.0.x), rather than the floating latest.

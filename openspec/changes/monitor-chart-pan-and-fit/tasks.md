## 1. Fix View & Panning

- [ ] 1.1 In `assets/monitor.js`, remove the zoom-plugin `limits` block from the chart options (it pinned the view to the window and blocked panning)
- [ ] 1.2 Remove the forced `scales.x.min = windowFrom` / `max = windowTo` from chart creation and from `selectWindow` (this caused right-edge bunching)
- [ ] 1.3 On a fresh window load only, fit the x-axis to the loaded data: set `scales.x.min`/`max` to the first/last loaded point timestamps (or leave unset to let Chart.js auto-fit) so the data fills the plot area
- [ ] 1.4 Ensure the zoom/pan refetch path (`renderChart` for non-fresh) swaps only dataset data via `update('none')` and never resets `scales.x.min`/`max`, so the gesture position is preserved
- [ ] 1.5 Keep `resetZoom` re-running the active window select (re-fits to that window's data); keep the never-destroy single-instance model

## 2. Pin Chart Dependencies

- [ ] 2.1 In `assets/monitor.html`, pin the CDN `<script>` tags for `chart.js`, `chartjs-plugin-zoom`, and `chartjs-adapter-date-fns` to exact versions that are verified to work together (e.g. Chart.js 4.4.x + zoom 2.0.x + adapter 3.0.x), replacing the floating major-version tags
- [ ] 2.2 Apply the same pinning to `assets/index.html` if it references Chart.js, for consistency

## 3. Verification

- [ ] 3.1 `cargo build` passes (assets re-embed); `node --check assets/monitor.js` passes
- [ ] 3.2 Browser check on the 30-day dataset (release build): the chart opens with data filling the plot area (not bunched); dragging pans the view and loads the newly visible range; wheel zoom-in densifies to raw and zoom-out coarsens to buckets; the gesture position is preserved across the refetch; reset returns to the active window; no console errors
- [ ] 3.3 Confirm the pinned library versions load (no 404s) and that zoom + pan both function with them

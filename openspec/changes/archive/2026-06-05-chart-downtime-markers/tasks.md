## 1. Classify outages

- [x] 1.1 In `assets/monitor.js`, add a `SUSTAINED_MS` constant (default ~60000) and classify each merged interval from `computeDownIntervals` as brief (`end - start < SUSTAINED_MS`) or sustained

## 2. Render markers

- [x] 2.1 Render a thin vertical `markLine` (fixed-pixel red) at every outage's start so any drop stays visible at any zoom (including when a sustained band would be sub-pixel)
- [x] 2.2 Render sustained outages as a `markArea` band (as today) with a `label` showing start time + duration (e.g. "14:03 · 1m40s"); do not draw a band for brief drops
- [x] 2.3 Make the markers interactive (remove `silent: true`) and wire hover to show the timestamp — single time for `markLine`, start→end + duration for the `markArea`
- [x] 2.4 Keep the existing line gap at outages unchanged

## 3. Verification

- [x] 3.1 `cargo build` (assets re-embed) and `node --check assets/monitor.js` pass
- [x] 3.2 (browser-only — needs your verification) Browser check: zoom in on a single dropped probe → thin marker stays visible; a multi-minute outage shows a fat labeled band; hovering either shows the timestamp/range; zoom far out → outage still locatable; no console errors
- [x] 3.3 (browser-only — tune after seeing it live) If deep-zoom marker density is noisy with the synthetic ~0.3% loss data, tune `SUSTAINED_MS` and/or suppress single-sample ticks; note the chosen values

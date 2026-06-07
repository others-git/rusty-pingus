## 1. Render the latency ribbon (assets/monitor.js)

- [x] 1.1 In `_renderHopChart`, replace the per-hop rectangle `renderItem` with a single custom series that builds one filled polygon: left edge at `x=-min_ms` (negative/centred axis), right edge at `x=+max_ms`, closed — using `api.coord` per hop so it connects across hop centers into a continuous ribbon.
- [x] 1.2 Break the ribbon at non-responding hops: emit a separate polygon per contiguous run of responding hops (don't bridge or zero-pinch the gap).
- [x] 1.3 Centred X axis `[-maxX, +maxX]`; average dots plotted at `+avg_ms` (positive side); `markLine` at `xAxis:0` as visual centre reference; min visual width enforced; tooltip unchanged.

## 2. Verification

- [x] 2.1 `node --check assets/monitor.js` passes.
- [ ] 2.2 Open a traceroute monitor: the min–max range renders as one continuous ribbon (not separate bars), averages overlay correctly within it, and a non-responding hop breaks the ribbon rather than pinching to 0.
- [ ] 2.3 Sync the `web-ui` main spec at archive time.

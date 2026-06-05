## 1. Fix tooltip formatter

- [x] 1.1 In `assets/monitor.js`, rewrite the chart `tooltip.formatter` to use `axisValue` for the timestamp and treat `value` as optional (only index it when it's an array); render "no response" when there is no value — removing the unconditional `params[0].value[0]` dereference

## 2. Verification

- [x] 2.1 `cargo build` (assets re-embed) and `node --check assets/monitor.js` pass
- [ ] 2.2 (browser-only — needs your verification) Browser check: hover across data points and over an outage gap — tooltip shows time + response (or "no response") with no console error

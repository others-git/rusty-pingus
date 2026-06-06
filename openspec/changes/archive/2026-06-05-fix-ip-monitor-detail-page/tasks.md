## 1. Anchor time windows to server data time (assets/monitor.js)

- [x] 1.1 Capture the latest server-recorded probe timestamp on the initial `/history` load in `init()` (e.g. `serverNowMs` from the newest row's `checked_at`), defaulting to `Date.now()` when there is no data.
- [x] 1.2 Update `serverNowMs` from live events (`last_checked_at`) and from each `loadRange` result, so the anchor tracks the newest data.
- [x] 1.3 Change `selectWindow` to anchor `windowTo` to `serverNowMs` (not `Date.now()`), with `windowFrom = windowTo − secs`.
- [x] 1.4 Audit other `Date.now()` uses that bound data ranges (e.g. the live "slide to now", `loadRange`'s `to = Math.min(toMs, Date.now())`) and switch them to the server anchor so they never clip server-stamped data.

## 2. Stop flashing on state-timeline pages

- [x] 2.1 In the live update handler, for `timelineKind` of `publicip`/`border`, update the header value live but do NOT call the per-probe chart reload.
- [x] 2.2 Track the currently displayed latest value (IP / fault class) and trigger a timeline reload only when an incoming event's value differs from it.
- [x] 2.3 Leave response-time monitors' live refresh behavior intact (still refresh per probe, now server-anchored).

## 3. Purpose-built public-IP view

- [x] 3.1 Verify the public-IP branch of the metrics strip shows no response-time value and uses IP-oriented tiles only (current IP, stable for, IP changes, last check). (Confirmed: strip gated `timelineKind !== 'publicip'`; public-IP strip shows IP tiles only.)
- [x] 3.2 Verify the chart header/labels for public-IP say "Public IP" (timeline), with no latency axis or "response time" wording, and remove any that leak. (Confirmed: header reads "Public IP"; timeline option has no latency axis; `(raw)`/loss summary gated `!timelineKind`. No leak found.)

## 4. Verification

- [ ] 4.1 With the server clock deliberately offset from the browser clock, open the public-IP detail page and confirm the IP timeline and "Current IP" populate (no longer empty / "no data").
- [ ] 4.2 Confirm the IP timeline does not flash while successive probes return the same IP, and that the header's "last check" still advances.
- [ ] 4.3 Change the observed IP (or simulate a differing `detail`) and confirm the timeline updates to show the change.
- [ ] 4.4 Confirm a response-time monitor's detail chart still renders and updates correctly (server-anchored), unaffected by the timeline-specific changes.
- [ ] 4.5 Confirm the public-IP view shows no response-time metric, axis, or label.
- [x] 4.6 Sync the `web-ui` and `live-status-updates` main specs via this change's deltas at archive time.

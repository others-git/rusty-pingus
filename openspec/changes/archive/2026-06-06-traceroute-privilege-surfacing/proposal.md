## Why

A traceroute monitor needs a raw ICMP socket (CAP_NET_RAW / elevated privileges). When that can't be opened, the probe already fails cleanly with a `privilege_error` reason — but in the UI it just appears as a generic "down", indistinguishable from a real network failure. A user has no way to tell that the monitor is misconfigured/under-privileged rather than the target being unreachable.

## What Changes

- **Surface traceroute unavailability distinctly in the UI.** When a traceroute monitor's latest probe failed because the raw socket couldn't be opened (privilege/socket error), the dashboard card and the detail page SHALL show a clear "traceroute unavailable — requires elevated privileges (CAP_NET_RAW)" state instead of a bare "down", including a short hint. Genuine network failures (timeouts, unreachable) keep their normal down presentation.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- **web-ui** — the dashboard and traceroute detail view surface a traceroute that cannot run due to missing privileges as a distinct, explained state rather than a generic "down".

## Impact

- **Backend:** none required — the probe already returns a stable `privilege_error` failure reason (and the monitor-list/history responses already carry `failure_reason`). Optionally make the reason text a touch more descriptive.
- **Frontend:** `assets/app.js`/`index.html` (detect a privilege/socket failure reason on a traceroute monitor and render a distinct "unavailable — needs privileges" indicator + tooltip on the card) and `assets/monitor.js`/`monitor.html` (a clear banner/notice on the traceroute detail page explaining it can't run and why).
- **No API, data, or config change.**
- **Specs:** `web-ui`.

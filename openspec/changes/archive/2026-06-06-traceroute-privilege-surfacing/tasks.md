## 1. Detect the unavailable state (assets/)

- [x] 1.1 Add a small shared helper that returns true when a monitor is a `traceroute` whose latest status is down with a privilege/socket `failure_reason` (`privilege_error`, `socket_error`, `join_error`). Centralize the token list (note the coupling to `probe/traceroute.rs`).

## 2. Surface on the dashboard (index.html / app.js)

- [x] 2.1 On a traceroute card in the unavailable state, render a distinct badge/icon (neutral/amber with a lock/warning icon) reading "unavailable" instead of the red "down" badge, with a tooltip: "Traceroute needs elevated privileges (CAP_NET_RAW)".

## 3. Surface on the detail page (monitor.html / monitor.js)

- [x] 3.1 On the traceroute detail page in the unavailable state, show a clear notice/banner explaining the monitor can't run and why (missing CAP_NET_RAW / elevated privileges), in a warning (not outage) tone.

## 4. Verification

- [x] 4.1 `node --check` passes for changed JS.
- [ ] 4.2 Run the app unprivileged with a traceroute monitor: the card and detail page show the distinct "unavailable — needs privileges" state (not a bare red "down").
- [ ] 4.3 Confirm a traceroute that fails for an ordinary reason (or a non-traceroute down monitor) still shows the normal down presentation.
- [ ] 4.4 Sync the `web-ui` main spec at archive time.

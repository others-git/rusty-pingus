## Why

The dashboard reflects status on a 30-second timer (poll of `/api/monitors`). With monitors that probe every few seconds — or sub-second — that feels stale: a monitor can go down and the dashboard won't show it for up to 30s. We want the dashboard to update in near real time, pushed from the server as each probe completes, while the monitor detail pages stay as they are (no auto-update).

## What Changes

- **Server pushes status as probes complete.** When the scheduler records a probe result, it broadcasts a lightweight status update on an in-process channel (name, status, response time, last-check time, failure reason, detail).
- **A streaming endpoint** `GET /api/monitors/stream` (Server-Sent Events) relays those updates to connected clients.
- **The dashboard subscribes** via `EventSource` and updates the matching monitor card in place the instant an update arrives — status, response time, "last checked", up/down counts.
- **Polling stays as a fallback/reconcile.** The existing periodic `/api/monitors` fetch is retained (it handles the initial load, refreshing 24h uptime, reconciling added/removed monitors, and keeps working if SSE is unavailable or drops). SSE just makes updates immediate between reconciles.
- **Monitor detail pages are unchanged** — they do not subscribe and do not auto-update (per request).

## Capabilities

### New Capabilities

- `live-status-updates`: Server-pushed, near-real-time monitor status to the dashboard via SSE, with polling as a graceful fallback.

## Impact

- `Cargo.toml` — add `tokio-stream` (features = ["sync"]) for `BroadcastStream` (already transitively present); SSE is built into Axum 0.7.
- `src/api/mod.rs` — a `StatusUpdate` payload; `AppState` gains a `broadcast::Sender<StatusUpdate>`; a `monitor_stream` SSE handler.
- `src/scheduler/mod.rs` — `run`/`run_probe` take the broadcast sender and emit a `StatusUpdate` after each successful insert (no extra DB query).
- `src/main.rs` — create the broadcast channel; pass the sender to the scheduler and into `AppState`.
- `src/web/mod.rs` — register `GET /api/monitors/stream`.
- `assets/app.js` — open an `EventSource`, apply updates to cards in place, keep the periodic poll as fallback/reconcile.
- No change to the monitor detail page or the chart.

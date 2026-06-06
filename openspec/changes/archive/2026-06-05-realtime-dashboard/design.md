## Context

The dashboard (`assets/app.js`, Alpine) loads `/api/monitors` once and re-polls every 30s (`setInterval`), updating reactive `monitors[]`. The scheduler (`src/scheduler/mod.rs`) runs a task per monitor; `run_probe` produces a `ProbeResult` and calls `db::insert_result`. The web server is Axum 0.7 with `AppState { pool, monitors }` threaded via `with_state`; routes live in `src/web/mod.rs`. `tokio` is "full" (so `broadcast` is available) and `tokio-stream` is already in the dependency tree. The detail page does not poll.

## Goals / Non-Goals

**Goals:** dashboard reflects status changes within ~a second of a probe completing, pushed from the server; robust (degrades to polling if SSE drops/unsupported); cheap (no per-probe DB work for the push); detail pages untouched.

**Non-Goals:** WebSockets (one-way push is all we need → SSE); pushing 24h uptime per probe (kept on the poll); auth on the stream (same trust model as the rest of the API); real-time on the detail page.

## Decisions

### 1. In-process broadcast on probe completion
Add `broadcast::Sender<StatusUpdate>` to `AppState` and pass it into the scheduler. After a successful `db::insert_result`, `run_probe` sends a `StatusUpdate` built directly from the `ProbeResult` (name, protocol, endpoint, status, response_time_ms, failure_reason, detail, checked_at) — **no extra query**. `send` errors (no subscribers) are ignored. A bounded channel (e.g. capacity 256); a lagging subscriber drops oldest events — fine, the poll reconciles. Alternative (DB NOTIFY / external bus) rejected: in-process broadcast is simplest and sufficient for a single-process app.

### 2. SSE endpoint `GET /api/monitors/stream`
Axum `Sse` response: subscribe to the broadcast, wrap the receiver in `tokio_stream::wrappers::BroadcastStream`, map each `StatusUpdate` to `axum::response::sse::Event::default().json_data(update)`, with `KeepAlive` (periodic comment) so proxies/idle connections don't time out. The stream ends when the client disconnects (Axum drops it) — no manual cleanup. Lag errors from `BroadcastStream` are skipped (filtered out), not surfaced as stream errors.

### 3. Dashboard consumes SSE, poll stays as fallback/reconcile
On init, `fetchMonitors()` (full list incl. uptime) as today, then open `EventSource('/api/monitors/stream')`. `onmessage`: parse the update, find the monitor in `monitors[]` by name, and update its live fields in place (`status`, `response_time_ms`, `last_checked_at`, `failure_reason`, `detail`); set `lastUpdated`; the up/down counts are derived getters so they recompute automatically. Keep the periodic `fetchMonitors()` poll (retain it; cadence can stay 30s) to: refresh 24h uptime, reconcile added/removed monitors, and serve as the fallback if SSE never connects. `EventSource` auto-reconnects on drop. Updates for a monitor name not currently in `monitors[]` (e.g. a just-added monitor not yet in the list) are ignored until the next poll adds it.

### 4. Detail page unchanged
The monitor detail page neither subscribes nor polls — explicitly out of scope (matches the request and current behavior).

## Risks / Trade-offs

- **Lagging/slow clients** miss some events (broadcast drops oldest) → reconciled by the retained poll; status is eventually consistent within the poll interval at worst.
- **24h uptime not pushed** → it refreshes on the poll, so the headline % can trail by the poll interval while status/response are live. Acceptable; pushing uptime per probe would add a DB query per probe per monitor.
- **Event volume** at low poll intervals (many monitors × sub-second) → small JSON per event; broadcast fan-out is cheap and only to connected dashboards. If it ever matters, throttle to status-changes-only or coalesce — noted, not needed for v1.
- **Proxy buffering of SSE** → `KeepAlive` mitigates; the poll fallback covers environments that buffer/strip SSE.
- **Browser-only verification** for the live behavior.

## Open Questions

- Whether to eventually push status-change events only (vs every probe) if event volume grows — defer until it's a problem.
- Whether to also stream to the detail page later — out of scope now.

## 1. Broadcast channel + payload

- [x] 1.1 Add `tokio-stream = { version = "0.1", features = ["sync"] }` to `Cargo.toml`
- [x] 1.2 In `src/api/mod.rs`, define `StatusUpdate { name, protocol, endpoint, status, response_time_ms, failure_reason, last_checked_at }` (Serialize; `detail` omitted — no such field on `ProbeResult`); add `updates: tokio::sync::broadcast::Sender<StatusUpdate>` to `AppState`
- [x] 1.3 In `src/main.rs`, create the broadcast channel (capacity ~256); put the sender in `AppState` and pass a clone to the scheduler

## 2. Emit updates from the scheduler

- [x] 2.1 Thread the broadcast sender into `scheduler::run` and `run_probe`
- [x] 2.2 After a successful `db::insert_result`, build a `StatusUpdate` from the `ProbeResult` (no extra query) and `send` it; ignore the no-subscribers error

## 3. SSE endpoint

- [x] 3.1 Add `monitor_stream` handler in `src/api/mod.rs`: subscribe to the broadcast, wrap in `tokio_stream::wrappers::BroadcastStream`, filter out lag errors, map each update to `axum::response::sse::Event::default().json_data(..)`, return `Sse::new(stream).keep_alive(KeepAlive::default())`
- [x] 3.2 Register `GET /api/monitors/stream` in `src/web/mod.rs` (before the `/*path` catch-all)

## 4. Dashboard consumes the stream

- [x] 4.1 In `assets/app.js`, after the initial `fetchMonitors()`, open `EventSource('/api/monitors/stream')`; on message, find the monitor in `monitors[]` by name and update its live fields in place (status, response_time_ms, last_checked_at, failure_reason, detail); set `lastUpdated`; ignore updates for unknown names
- [x] 4.2 Keep the periodic `fetchMonitors()` poll as fallback + reconcile (uptime refresh, add/remove membership); ensure `EventSource` reconnects are handled (default auto-reconnect) and the connection is closed on teardown
- [x] 4.3 Do not touch `monitor.js` / the detail page (it stays non-auto-updating)

## 5. Verification

- [x] 5.1 `cargo build` and `cargo clippy -- -D warnings` pass; `cargo test` passes
- [x] 5.2 Smoke test the stream: with a monitor running, `curl -N http://127.0.0.1:3000/api/monitors/stream` shows events arriving as probes complete; `GET /api/monitors/stream` stays open (keep-alive)
- [ ] 5.3 Browser check: open the dashboard, flip a monitor up/down (e.g. add a TCP monitor to a port you stop/start) → the card updates within ~1s without reload; kill the stream (or block it) → dashboard still updates via the poll; detail page does not auto-update
- [x] 5.4 Update `README.md` API list with `GET /api/monitors/:name` … and the new `GET /api/monitors/stream` (SSE live status)

## 1. Config: enabled flag (monitors/mod.rs)

- [x] 1.1 Add `enabled: bool` to every monitor config struct, defaulting to `true` and `skip_serializing_if` "is true" so existing/enabled configs omit the key.
- [x] 1.2 Add `enabled` to each `Raw*MonitorConfig` with `#[serde(default = "default_true")]` and carry it through the `From` impls (add a `fn default_true() -> bool { true }`).
- [x] 1.3 Add a `MonitorConfig::enabled(&self) -> bool` accessor (matching the `name()`/`interval_ms()` pattern).
- [x] 1.4 Add `MonitorStore::set_enabled(&self, name: &str, enabled: bool) -> Result<bool>` that updates the monitor, persists, and notifies (returns false if not found).
- [x] 1.5 Add the `enabled` example to `DEFAULT_MONITORS_CONFIG` comments; unit test: missing key → enabled; `enabled = false` round-trips.

## 2. Scheduler: honor enabled + live toggle (scheduler/mod.rs)

- [x] 2.1 In the initial spawn loop, only spawn a task for monitors where `enabled()` is true.
- [x] 2.2 In `diff_and_reload`, reconcile desired vs. actual: for each new monitor, if `enabled()` and no task exists → spawn; if not `enabled()` and a task exists → cancel/remove. Keep the existing removed-name cancellation.

## 3. API: toggle endpoint + expose enabled (api/mod.rs, web/mod.rs)

- [x] 3.1 Add `enabled: bool` to `MonitorStatus`, populated from config in `list_monitors`.
- [x] 3.2 Add a handler `set_monitor_enabled` for `POST /api/monitors/:name/enabled` taking `{ "enabled": bool }`, calling `MonitorStore::set_enabled`; return 200 + updated list, or 404 if unknown.
- [x] 3.3 Register the route in `web/mod.rs`.

## 4. Frontend: toggle control + paused state (assets/)

- [x] 4.1 In `app.js`, add a `toggleMonitor(name, enabled)` action that POSTs to the endpoint and updates the card's `enabled` in place.
- [x] 4.2 In `index.html`, add a per-card enable/disable toggle control bound to `m.enabled`.
- [x] 4.3 Render a disabled monitor in a de-emphasized "paused" state (dimmed card + "paused" badge) distinct from "down"; drive it from `m.enabled`, not live events.
- [x] 4.4 Ensure the add-monitor form creates monitors enabled by default (no UI field needed unless trivial).

## 5. Verification

- [x] 5.1 `cargo build` + `cargo test` pass.
- [x] 5.2 Create a monitor, disable it via the API: confirm probing stops (no new probe rows) and the config persists `enabled = false`; re-enable and confirm probing resumes — all without restarting.
- [x] 5.3 Confirm an existing `monitors.toml` without `enabled` keys loads all monitors as enabled.
- [~] 5.4 Confirm the dashboard shows the toggle and renders a disabled monitor as paused (distinct from down), reflecting state from the list.
- [x] 5.5 Sync the `monitor-config`, `scheduler`, `monitor-crud-api`, and `web-ui` main specs via this change's deltas at archive time.

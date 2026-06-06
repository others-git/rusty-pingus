## Why

Right now the only way to stop a monitor from probing is to delete it — which throws away its config and history. Users want to pause a monitor temporarily (during maintenance, a known outage, or while tuning) and resume it later without losing anything. An enable/disable toggle provides that.

## What Changes

- **Per-monitor `enabled` flag.** Each monitor gains an `enabled` boolean (defaults to `true`, omitted in config when true for back-compat). A disabled monitor is **not probed**: the scheduler runs no probe task for it.
- **Toggle without losing data.** Disabling keeps the monitor's config and stored history; re-enabling resumes probing. No history is purged on disable.
- **Scheduler honors toggles live.** Via the existing hot-reload path, disabling a monitor stops its running probe task and enabling one starts it — no restart needed.
- **API to toggle.** A new endpoint sets a monitor's enabled state; the monitor-list response includes each monitor's `enabled` flag.
- **Dashboard control + state.** Each monitor card shows a toggle; a disabled monitor renders in a distinct, de-emphasized "disabled/paused" state (not as "down"), and its status badge reflects that it is paused rather than failing.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- **monitor-config** — monitors carry an `enabled` flag (default true; back-compatible with files that omit it).
- **scheduler** — disabled monitors are not scheduled; enabling/disabling takes effect via hot-reload without a restart.
- **monitor-crud-api** — a new endpoint toggles a monitor's enabled state, and the monitor-list response exposes `enabled`.
- **web-ui** — the dashboard shows a per-monitor enable/disable toggle and a distinct disabled (paused) state.

## Impact

- **Backend (`src/`):** `monitors/mod.rs` (add `enabled` to each config variant + `Raw*` default-true, a `MonitorConfig::enabled()` accessor, and a `MonitorStore::set_enabled(name, bool)` that persists + notifies), `scheduler/mod.rs` (skip disabled monitors when spawning; in `diff_and_reload`, stop tasks that became disabled and start tasks that became enabled), `api/mod.rs` (toggle endpoint + `enabled` in `MonitorStatus`), `web/mod.rs` (route).
- **Frontend (`assets/`):** `app.js` (toggle action calling the endpoint; reflect `enabled` in cards), `index.html` (toggle control + disabled styling), and the add-monitor form may default new monitors to enabled.
- **Config:** `monitors.toml` entries may include `enabled = false`; existing files (no `enabled` key) load as enabled. No migration needed.
- **No database/schema change.**
- **Specs:** `monitor-config`, `scheduler`, `monitor-crud-api`, `web-ui`.

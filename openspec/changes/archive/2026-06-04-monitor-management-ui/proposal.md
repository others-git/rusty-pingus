## Why

Currently, adding or removing monitors requires manually editing `config.toml` and restarting the binary. This is a friction point for non-technical users and makes the web dashboard feel incomplete. The web UI should be the primary way to manage monitors — no file editing required.

Splitting the configuration into two files — `config.toml` for application settings (bind address, database path, logging) and `monitors.toml` for monitor definitions — makes both cleaner and enables live reloading of monitors without restarting the entire service.

## What Changes

- Split config into two files:
  - `config.toml` — app-level settings only (`[web]`, `[database]`, `[defaults]`). Monitor entries are removed.
  - `monitors.toml` — monitor definitions only (`[[monitors]]` entries). Created automatically if absent; default includes commented examples.
- **New API endpoints** for monitor CRUD:
  - `GET /api/monitors/config` — return all configured monitors
  - `POST /api/monitors` — add a new monitor (validated)
  - `DELETE /api/monitors/:name` — remove a monitor by name
- **Web UI: Add Monitor form** on the dashboard — modal or slide-in panel with fields for name, protocol, and protocol-specific settings (URL / host+port / host); interval; timeout. Inline validation errors.
- **Live reload**: after a write to `monitors.toml`, the scheduler restarts affected monitor tasks without a full binary restart.
- Input validation on the API: required fields, URL format, port range, interval/timeout bounds, unique name enforcement.

## Capabilities

### New Capabilities

- `monitor-store`: Read and write `monitors.toml`; the authoritative source for monitor definitions at runtime
- `monitor-crud-api`: REST endpoints to list, add, and delete monitors, with validation
- `monitor-hot-reload`: Reload monitor tasks from the updated `monitors.toml` without restarting the binary

### Modified Capabilities

- `monitor-config`: Config file handling changes — monitors are no longer read from `config.toml`; they come from `monitors.toml`
- `web-ui`: Dashboard gains an "Add Monitor" button and form; monitor cards gain a delete button

## Impact

- `src/config/mod.rs` — `Config` struct removes `monitors` field; `MonitorConfig` types move to a new `src/monitors/mod.rs`
- New `src/monitors/mod.rs` — `MonitorStore` that owns the `monitors.toml` path, provides load/save/add/remove, wrapped in `Arc<RwLock<>>`
- `src/api/mod.rs` — 3 new route handlers; `AppState` gains `MonitorStore`
- `src/scheduler/mod.rs` — `run()` accepts a `MonitorStore` and supports hot-reload via a watch channel
- `assets/index.html`, `assets/app.js` — Add Monitor button, modal form, delete buttons
- Existing `config.toml` / `monitors.toml` migration: on startup, if `config.toml` contains `[[monitors]]` entries, they are automatically migrated to `monitors.toml` and removed from `config.toml`

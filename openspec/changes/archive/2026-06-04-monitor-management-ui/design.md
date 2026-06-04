## Context

Rusty-pingus currently loads all configuration — app settings and monitor definitions — from a single `config.toml`. The monitors array is parsed once at startup and passed to the scheduler. There is no mechanism to modify monitors at runtime. The web UI is read-only.

This change adds write capability: a second file (`monitors.toml`) owns monitor definitions, a `MonitorStore` provides thread-safe read/write access, new API endpoints expose CRUD operations, and the scheduler supports adding/removing monitor tasks without a full restart.

## Goals / Non-Goals

**Goals:**
- `monitors.toml` as the live source of truth for monitor definitions
- API: list, add, delete monitors (no edit in v1 — delete + re-add covers that)
- Web UI form for adding monitors with inline validation
- Delete button per monitor card in the dashboard
- Hot-reload: scheduler picks up changes without restarting the binary
- Auto-migration: existing `[[monitors]]` in `config.toml` moved to `monitors.toml` on first run
- Input validation: required fields, URL syntax, port range 1–65535, interval ≥5s, timeout ≥1s and < interval, unique names

**Non-Goals:**
- Edit/update existing monitor (delete + re-add)
- Monitor grouping or tagging
- Authentication on the management API (same as the rest of the UI — trusted network assumed)
- Bulk import/export
- Testing a monitor before saving it

## Decisions

### 1. `MonitorStore` as `Arc<RwLock<MonitorStoreInner>>`
Multiple async tasks read monitor configs (scheduler) and write them (API handlers). A `tokio::sync::RwLock` inside an `Arc` provides safe concurrent access. The store holds the current list in memory and the path to `monitors.toml`. Alternative: a channel-based actor — more complex, unnecessary at this scale.

### 2. Hot-reload via `tokio::watch` channel
`MonitorStore::save()` sends on a `watch::Sender<Vec<MonitorConfig>>`. The scheduler holds a `watch::Receiver` and detects changes. On change, it diffs the old and new lists: spawns tasks for added monitors, cancels tasks for removed ones. Alternative: restart all tasks on any change — simpler but causes a brief gap in monitoring for unchanged monitors.

### 3. New `AppState` struct replacing bare `SqlitePool`
Both the API handlers and the static asset handler currently use `SqlitePool` as state. Adding `MonitorStore` means we need a composite state. Extract `AppState { pool: SqlitePool, monitors: MonitorStore }` and pass it through Axum's `with_state`. All handlers updated accordingly.

### 4. Config split strategy
`config.toml` keeps `[web]`, `[database]`, `[defaults]`. `monitors.toml` keeps `[[monitors]]`. On startup: if `config.toml` contains monitors, migrate them to `monitors.toml` (append or create) and rewrite `config.toml` without the monitor entries. This is a one-time migration; afterwards the files are independent.

### 5. API validation at the handler layer
Validation rules enforced in the `POST /api/monitors` handler before writing to the store:
- `name`: non-empty, max 100 chars, no path separators (`/`, `\`), unique across existing monitors
- HTTP: `url` is a valid `http://` or `https://` URL
- TCP: `host` is non-empty, `port` is 1–65535
- ICMP: `host` is non-empty
- `interval_secs`: 5–86400
- `timeout_secs`: 1–interval_secs
Returns `422 Unprocessable Entity` with a structured `{ "errors": [...] }` JSON body on failure. Alternative: serde validation crate — adds complexity for a simple case; hand-rolled is sufficient.

### 6. Frontend: Alpine.js modal form
The "Add Monitor" button opens a modal with protocol selector tabs (HTTP / TCP / ICMP) and conditional fields. Alpine.js `x-show` handles field visibility. On submit, `POST /api/monitors` is called; validation errors are displayed inline. On success, the modal closes and the monitor list refreshes. Alternative: separate page — breaks the single-page flow.

### 7. monitors.toml default content
Same commented-example approach as `config.toml` default: if `monitors.toml` doesn't exist, create it with commented examples. On first install without any monitors, the file exists but is empty (all entries commented), so the scheduler starts with zero monitors.

## Risks / Trade-offs

- **Race condition on file write**: two concurrent API calls could both read, modify, and write `monitors.toml`. Mitigated by `tokio::sync::RwLock` — writes are serialized.
- **TOML rewrite loses comments**: reading and re-serializing `monitors.toml` via `serde` will lose any hand-written comments. Acceptable for a managed file; document this behaviour.
- **Diff-based hot-reload complexity**: comparing old vs new monitor lists requires matching by name. If a monitor is renamed (delete + re-add with same settings but new name), the old task is cancelled and a new one spawned — a brief gap in monitoring. Acceptable for v1.
- **Migration is one-way**: once monitors are migrated to `monitors.toml`, they can't be moved back to `config.toml` automatically. Document this clearly in the README.

## Open Questions

- Should `DELETE /api/monitors/:name` require confirmation in the UI? → Yes — a simple confirm dialog in the frontend (no server-side change needed).
- Should the API return the full updated monitor list after a successful POST or DELETE? → Yes, return the new list to avoid a separate refetch.

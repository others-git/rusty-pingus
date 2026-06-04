## 1. Config Split & Monitor Store

- [x] 1.1 Remove `monitors: Vec<MonitorConfig>` from `Config` struct in `src/config/mod.rs`; add `monitors_path: String` field (default `./monitors.toml`)
- [x] 1.2 Create `src/monitors/mod.rs` with `MonitorStore` struct: holds `Arc<RwLock<Vec<MonitorConfig>>>`, the file path, and a `watch::Sender<Vec<MonitorConfig>>`
- [x] 1.3 Implement `MonitorStore::load(path) -> Result<(MonitorStore, watch::Receiver<Vec<MonitorConfig>>)>`: reads `monitors.toml`, falls back to default if missing, returns store + receiver
- [x] 1.4 Implement `MonitorStore::save(&self) -> Result<()>`: serialises current list to TOML and atomically writes (write tmp file, rename)
- [x] 1.5 Implement `MonitorStore::add(&self, monitor: MonitorConfig) -> Result<()>`: acquires write lock, checks name uniqueness, appends, saves, sends on watch channel
- [x] 1.6 Implement `MonitorStore::remove(&self, name: &str) -> Result<bool>`: acquires write lock, removes by name, saves, sends on watch channel; returns `false` if not found
- [x] 1.7 Implement `MonitorStore::list(&self) -> Vec<MonitorConfig>`: acquires read lock, returns a clone of the current list
- [x] 1.8 Add `DEFAULT_MONITORS_CONFIG` constant (analogous to `DEFAULT_CONFIG`) with commented HTTP/TCP/ICMP examples
- [x] 1.9 Implement config migration in `main.rs`: if `config.toml` has monitors, write them to `monitors.toml` (if it doesn't already exist), rewrite `config.toml` without the monitors entries, and log a warning
- [x] 1.10 Add `monitors` module to `src/lib.rs`

## 2. AppState & Dependency Wiring

- [x] 2.1 Define `AppState { pool: SqlitePool, monitors: MonitorStore }` in `src/api/mod.rs` (or a new `src/state.rs`)
- [x] 2.2 Update `web::serve(bind, pool, cancel)` signature to `web::serve(bind, state: AppState, cancel)` and thread `AppState` through the Axum router via `with_state`
- [x] 2.3 Update all existing API handlers (`list_monitors`, `monitor_history`, `monitor_uptime`) to extract `State(state): State<AppState>` and use `state.pool`
- [x] 2.4 Update `main.rs`: construct `AppState` after loading the monitor store; pass it to `web::serve`

## 3. CRUD API Endpoints

- [x] 3.1 Implement `GET /api/monitors/config` handler: return `state.monitors.list()` as JSON
- [x] 3.2 Implement `POST /api/monitors` handler: deserialise body, run validation, call `state.monitors.add()`, return 201 + updated list
- [x] 3.3 Implement validation logic for `POST /api/monitors`: name required + max 100 chars + no `/\` chars; HTTP: valid URL; TCP: port 1–65535; ICMP: host non-empty; interval ≥5; timeout ≥1 and < interval; duplicate name check against `state.monitors.list()`
- [x] 3.4 Implement `DELETE /api/monitors/:name` handler: call `state.monitors.remove(name)`, return 200 + updated list or 404 if not found
- [x] 3.5 Register the three new routes in the Axum router (before the `/*path` catch-all)

## 4. Scheduler Hot-Reload

- [x] 4.1 Update `scheduler::run()` to accept a `watch::Receiver<Vec<MonitorConfig>>` alongside the `CancellationToken`
- [x] 4.2 In the scheduler main loop, use `tokio::select!` to watch for either a cancellation signal or a change on the watch receiver
- [x] 4.3 On watch change: diff the new list against the currently running tasks (keyed by monitor name); cancel tasks for removed monitors; spawn new tasks for added monitors
- [x] 4.4 Use a `HashMap<String, CancellationToken>` to track per-monitor cancel tokens so individual tasks can be stopped cleanly
- [x] 4.5 Update `main.rs` to pass the watch receiver to `scheduler::run()`

## 5. Dashboard — Delete Button

- [x] 5.1 Add a delete button to each monitor card in `assets/index.html`: a small trash icon (Font Awesome `fa-trash`) visible on hover, with `x-on:click` that sets a `pendingDelete` name in Alpine state
- [x] 5.2 Add a confirm dialog (Alpine-driven, inline in the page) that appears when `pendingDelete` is set, asking "Delete monitor <name>?"; on confirm calls `deleteMonitor(name)`, on cancel clears `pendingDelete`
- [x] 5.3 Add `deleteMonitor(name)` async method to `dashboard()` Alpine component in `assets/app.js`: calls `DELETE /api/monitors/:name`, on success removes the monitor from the local `monitors` array and clears `pendingDelete`

## 6. Dashboard — Add Monitor Modal

- [x] 6.1 Add "Add Monitor" button in the sticky header in `assets/index.html` (cyan, Font Awesome `fa-plus` icon)
- [x] 6.2 Create the modal markup in `assets/index.html`: dark overlay + centered card, `x-show="showAddModal"`, dismissible by clicking overlay or pressing Escape (`x-on:keydown.escape.window`)
- [x] 6.3 Add protocol selector tabs (HTTP / TCP / ICMP) in the modal using Alpine `:class` active state
- [x] 6.4 Add HTTP fields: Name, URL, Interval (secs), Timeout (secs), Method (select: GET/POST/HEAD), Expected Status (optional)
- [x] 6.5 Add TCP fields: Name, Host, Port, Interval (secs), Timeout (secs)
- [x] 6.6 Add ICMP fields: Name, Host, Interval (secs), Timeout (secs)
- [x] 6.7 Use `x-show="form.protocol === 'http'"` etc. to show only the relevant field group
- [x] 6.8 Add per-field error display: `<p x-show="errors.url" x-text="errors.url" class="text-red-400 text-xs mt-1"></p>` pattern for each validated field
- [x] 6.9 Add `addMonitorForm` Alpine data object and `submitMonitor()` method in `assets/app.js`: builds request body from form state, calls `POST /api/monitors`, maps 422 `errors` array to per-field error display, on 201 appends to `monitors` and closes modal

## 7. Verification

- [x] 7.1 `cargo build` passes cleanly; `cargo clippy -- -D warnings` passes
- [x] 7.2 `cargo test` passes (update any tests that use the old `Config` with monitors field)
- [x] 7.3 Smoke test: start with no `monitors.toml`; confirm default generated; add an HTTP monitor via UI; confirm it appears in dashboard and begins probing; delete it; confirm it disappears
- [x] 7.4 Smoke test: start with monitors in legacy `config.toml`; confirm migration writes `monitors.toml` and `config.toml` is rewritten without monitors
- [x] 7.5 Smoke test validation: try to add monitor with empty name, bad URL, port 0, interval 2; confirm 422 errors shown in form
- [x] 7.6 Update `README.md`: document the config split (`config.toml` vs `monitors.toml`), the `--monitors` CLI flag, and migration behaviour

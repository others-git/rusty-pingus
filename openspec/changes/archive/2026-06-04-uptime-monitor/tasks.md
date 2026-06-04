## 1. Project Setup

- [x] 1.1 Initialize Cargo workspace with `cargo new --bin rusty-pingus`
- [x] 1.2 Add all dependencies to `Cargo.toml`: `tokio`, `axum`, `sqlx`, `reqwest`, `serde`, `toml`, `clap`, `surge-ping`, `rust-embed`, `tracing`, `tracing-subscriber`, `anyhow`, `chrono`
- [x] 1.3 Create project directory structure: `src/config`, `src/probe`, `src/db`, `src/scheduler`, `src/web`, `src/api`, `assets/`
- [x] 1.4 Set up `tracing` logging with configurable log level via env var

## 2. Configuration

- [x] 2.1 Define `Config`, `MonitorConfig`, `HttpMonitorConfig`, `TcpMonitorConfig`, `IcmpMonitorConfig`, and `Defaults` structs in `src/config/mod.rs` with `serde::Deserialize`
- [x] 2.2 Implement TOML config file loading with error reporting for missing/malformed files
- [x] 2.3 Implement defaults merging: apply `[defaults]` values to monitors that don't override them
- [x] 2.4 Add `clap` CLI with `--config` flag (default: `./config.toml`), `--bind` flag, and `--db` flag
- [x] 2.5 Write a sample `config.toml` with examples of each monitor type

## 3. Database & Migrations

- [x] 3.1 Set up `sqlx` with SQLite feature and create `migrations/` directory
- [x] 3.2 Write migration `001_initial.sql`: create `probe_results` table with columns `id`, `monitor_name`, `protocol`, `endpoint`, `status`, `response_time_ms`, `failure_reason`, `checked_at`
- [x] 3.3 Implement `db::init(path)` that opens the SQLite connection pool with WAL mode enabled and runs pending migrations
- [x] 3.4 Implement `db::insert_result(pool, result)` for persisting probe results
- [x] 3.5 Implement `db::get_current_status(pool)` returning the latest result per monitor
- [x] 3.6 Implement `db::get_history(pool, monitor_name, from, to, limit)` for paginated history queries
- [x] 3.7 Implement `db::get_uptime(pool, monitor_name, window_secs)` calculating uptime percentage
- [x] 3.8 Implement `db::prune_old_results(pool, retention_days)` for the retention policy cleanup task

## 4. Probe Engine

- [x] 4.1 Define `ProbeResult` struct in `src/probe/mod.rs` with all required fields (`monitor_name`, `protocol`, `endpoint`, `status`, `response_time_ms`, `failure_reason`, `checked_at`)
- [x] 4.2 Implement `probe::http::run(config) -> ProbeResult` using `reqwest`: handle timeout, connection refused, TLS errors, and unexpected status codes
- [x] 4.3 Implement `probe::tcp::run(config) -> ProbeResult` using `tokio::net::TcpStream::connect_timeout`: handle timeout and DNS errors
- [x] 4.4 Implement `probe::icmp::run(config) -> ProbeResult` using `surge-ping`: handle no-reply and log privilege errors
- [x] 4.5 Add privilege check for ICMP at startup: log a warning and mark ICMP monitors as errored if CAP_NET_RAW is unavailable
- [x] 4.6 Write unit tests for each probe type using mock targets or local loopback

## 5. Scheduler

- [x] 5.1 Implement `scheduler::start(monitors, db_pool)` that spawns a `tokio::task` per monitor
- [x] 5.2 Each task SHALL run the initial probe immediately on spawn, then loop with `tokio::time::interval`
- [x] 5.3 Wrap each probe task with `tokio::task::spawn` and catch panics; log errors and restart the task
- [x] 5.4 Implement graceful shutdown using `tokio::signal` for SIGTERM/SIGINT and a `CancellationToken` (or broadcast channel) to signal tasks to stop
- [x] 5.5 Implement drain timeout: after cancellation signal, wait up to 10 seconds for tasks to finish before force-exiting

## 6. Web Frontend Assets

- [x] 6.1 Create `assets/index.html`: dashboard page with a monitors grid/list
- [x] 6.2 Create `assets/monitor.html`: detail page with history table and response time chart (use Chart.js via CDN)
- [x] 6.3 Create `assets/app.js`: fetch `/api/monitors` every 30 seconds and update DOM; handle `pending` status display
- [x] 6.4 Create `assets/monitor.js`: fetch history and uptime data for a specific monitor; render Chart.js line chart
- [x] 6.5 Create `assets/style.css`: minimal responsive styles; green/red status indicators

## 7. Web API & Server

- [x] 7.1 Implement `GET /api/monitors` handler returning JSON array of current monitor statuses with uptime_24h
- [x] 7.2 Implement `GET /api/monitors/:name/history` handler with `from`, `to`, `limit` query params; return 404 for unknown monitor
- [x] 7.3 Implement `GET /api/monitors/:name/uptime` handler returning uptime percentages for 1h, 24h, 7d, 30d windows
- [x] 7.4 Set up `rust-embed` to embed `assets/` directory into the binary
- [x] 7.5 Implement static asset serving route: serve `index.html` at `/`, `monitor.html` at `/monitors/:name` (client-side routing), and other assets at their paths
- [x] 7.6 Wire Axum router with all API routes and static asset fallback
- [x] 7.7 Start Axum server on the configured bind address as a separate `tokio::task` alongside the scheduler

## 8. Integration & Hardening

- [x] 8.1 Wire everything together in `main.rs`: parse CLI args → load config → init DB → start scheduler → start web server → await shutdown signal
- [x] 8.2 Implement periodic retention cleanup: spawn a task that runs `db::prune_old_results` once per day if retention is configured
- [x] 8.3 Add integration test: start binary with a test config (HTTP probe against `httpbin.org` or a local mock), verify a result is stored in the DB
- [x] 8.4 Verify binary builds and runs correctly on Linux; document privilege requirements for ICMP in `README.md`
- [x] 8.5 Write `README.md` with installation, configuration reference (all config fields with types and defaults), and example `config.toml`

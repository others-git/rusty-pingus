## Why

Teams need visibility into whether their external services and endpoints are reachable from a specific network location. Rusty-Pingus provides a self-hosted, lightweight uptime monitor written in Rust that actively probes endpoints using configurable protocols and exposes results via a web interface — without relying on third-party SaaS monitoring services.

## What Changes

- New Rust binary `rusty-pingus` with a multi-protocol probe engine (HTTP, TCP, ICMP)
- TOML/YAML configuration file for defining monitors, protocols, intervals, and thresholds
- Background scheduler that runs probes at configurable per-monitor intervals
- SQLite database for persisting probe results and uptime history
- Web interface (served by the binary) for viewing monitor status, uptime percentages, and response time history
- Alert/notification hooks for status changes (up → down, down → up)

## Capabilities

### New Capabilities

- `monitor-config`: Define and manage monitors via a configuration file (endpoint, protocol, interval, thresholds)
- `probe-engine`: Execute TCP, HTTP, and ICMP probes against configured endpoints and record results
- `result-storage`: Persist probe results and uptime history in SQLite
- `web-ui`: Serve a web dashboard showing current monitor status, uptime %, and response time charts
- `scheduler`: Schedule and run probes at per-monitor configurable intervals

### Modified Capabilities

## Impact

- New Rust project with dependencies: `tokio` (async runtime), `reqwest` (HTTP probes), `sqlx` (SQLite), `axum` (web server), `serde`/`serde_json` (config + API), `clap` (CLI), `ping`/`surge-ping` (ICMP)
- SQLite database file created at a configurable path (default: `./data/rusty-pingus.db`)
- Config file read at startup (default: `./config.toml`)
- Web server listens on a configurable host/port (default: `0.0.0.0:3000`)

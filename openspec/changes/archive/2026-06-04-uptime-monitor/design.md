## Context

Rusty-Pingus is a new greenfield Rust project with no existing codebase. The goal is a single self-contained binary that an operator can drop onto any Linux/macOS host and start monitoring external endpoints immediately. It must run reliably as a background process with minimal resource overhead.

Key constraints:
- Single binary deployment (no separate DB server, no node/python runtime)
- SQLite for persistence (embedded, zero-infra)
- Async Rust via Tokio for concurrent probe execution
- Web interface served from within the binary (no separate frontend build pipeline at runtime)

## Goals / Non-Goals

**Goals:**
- Multi-protocol probing: HTTP(S), TCP, ICMP
- Per-monitor configurable intervals and timeouts
- Persistent result history in SQLite
- Web dashboard (status overview, uptime %, response time history)
- Single-binary, zero-dependency deployment

**Non-Goals:**
- Distributed/clustered monitoring (single-node only)
- Push alerting integrations (Slack, PagerDuty) in v1
- Agent-based monitoring (remote agents reporting back)
- Authentication/authorization on the web UI in v1
- SLA reporting or complex aggregations beyond basic uptime %

## Decisions

### 1. Async runtime: Tokio
Tokio is the de-facto standard for async Rust. `reqwest` (HTTP), `sqlx` (SQLite async), and `axum` (web) all build on Tokio natively. Alternative: `async-std` — rejected because ecosystem integration is weaker.

### 2. Web framework: Axum
Axum is ergonomic, well-maintained, and Tokio-native. It lets us serve both a JSON API and static assets from the same binary. Alternative: `actix-web` — viable but heavier and more complex for our use case.

### 3. Database: SQLite via sqlx
`sqlx` provides async SQLite access with compile-time checked queries. The database is a single file, making backup and portability trivial. Migrations managed with `sqlx migrate`. Alternative: `rusqlite` (sync) — rejected; blocking DB calls in an async runtime require spawn_blocking overhead.

### 4. Frontend: Embedded static assets
HTML/CSS/JS assets compiled into the binary using `rust-embed`. This keeps deployment to a single file. The UI will be a lightweight SPA (vanilla JS or minimal framework like Preact) that fetches from a JSON API served by Axum. Alternative: SSR with a templating engine — simpler but harder to make interactive (charts, live updates).

### 5. Configuration: TOML file
TOML is Rust-idiomatic (`serde` + `toml` crate), human-friendly, and well-understood. Config is loaded at startup; monitors can be added by editing the file and restarting (or via a future hot-reload path). Alternative: YAML — more familiar to ops teams but more footguns (Norway problem, etc.).

### 6. ICMP probing: surge-ping
Raw ICMP requires elevated privileges (CAP_NET_RAW or root). `surge-ping` provides an async ICMP implementation. We document the privilege requirement clearly; TCP/HTTP probes work without elevation. Alternative: shell out to system `ping` — fragile and platform-dependent.

### 7. Scheduler: tokio tasks per monitor
Each monitor runs as an independent `tokio::task` looping at its configured interval. This is simple, requires no external scheduler, and scales to hundreds of monitors comfortably. Alternative: a single scheduler loop with a priority queue — more complex with minimal benefit at this scale.

## Risks / Trade-offs

- **ICMP requires privileges** → Document clearly; provide fallback TCP probe as alternative for unprivileged deployments.
- **SQLite write contention** → All probe tasks write results concurrently. Mitigate with WAL mode (`PRAGMA journal_mode=WAL`) and a small connection pool (sqlx `PoolOptions`).
- **Embedded assets increase binary size** → Acceptable for a monitoring tool; gzip compression via `rust-embed` keeps it manageable.
- **No hot-reload of config** → Operators must restart to pick up changes. Acceptable for v1; a SIGHUP handler can be added later.
- **No auth on web UI** → Acceptable if the service is only exposed on localhost or a private network. Document this clearly.

## Migration Plan

This is a new project with no existing data:
1. `cargo build --release` produces the binary
2. Operator places `config.toml` alongside the binary
3. On first run, `sqlx migrate run` initializes the SQLite schema automatically
4. Run as a systemd service or process manager (pm2, supervisor)

Rollback: stop the process, replace binary with previous version, restart.

## Open Questions

- Should HTTP probes support custom headers and request bodies (for authenticated endpoints)? → Assume yes, include in spec.
- Should the web UI support real-time push (WebSocket/SSE) for live status updates, or is polling sufficient for v1? → Polling (5s interval) for v1; SSE can be added later.

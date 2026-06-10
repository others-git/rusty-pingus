# rusty-pingus

Self-hosted uptime monitor written in Rust. Monitors external endpoints via HTTP, TCP, ICMP, public-IP, and border checks from your own network location. Stores results in SQLite and serves a web dashboard.

## Features

- HTTP(S), TCP, ICMP, public-IP, border, and traceroute probing
- Per-monitor configurable intervals and timeouts
- SQLite persistence with automatic migrations
- Web dashboard at `http://localhost:3000`
- Monitor detail chart (Apache ECharts) with built-in zoom/pan and a range slider, dynamic resolution (loads finer detail as you zoom in, down to individual probes), and downtime shown as a gap in the line plus a red band
- JSON API for programmatic access
- Single self-contained binary
- **Windows**: system tray icon, double-click to open dashboard, no console window
- Add and remove monitors via the web UI — no config file editing required
- Auto-generates default config files on first run

## Installation

```bash
cargo build --release
cp target/release/rusty-pingus /usr/local/bin/
```

## Configuration

Configuration is split into two files:

| File | Purpose |
|---|---|
| `config.toml` | App settings: bind address, database path, defaults, log retention |
| `monitors.toml` | Monitor definitions: managed by the web UI or edited directly |

Both files are auto-generated with commented examples on first run.

### Timing units (milliseconds)

Monitor `interval` and `timeout` are configured in **milliseconds** via the `interval_ms` and `timeout_ms` fields. Validation requires `interval_ms ≥ 5000`, `timeout_ms ≥ 1000`, and `timeout_ms < interval_ms`.

Older files that use the legacy `interval_secs` / `timeout_secs` keys are still accepted: their values are converted to milliseconds (×1000) on startup, the file is rewritten in the `*_ms` form, and a warning is logged. When both forms are present for the same field, the `*_ms` value wins.

### Migration from single config.toml

If you have an existing `config.toml` with `[[monitors]]` entries (from v0.0.1), rusty-pingus will automatically migrate them to `monitors.toml` on startup and remove the entries from `config.toml`.

### config.toml

App-wide settings only — monitors live in `monitors.toml` (see below). See `config.toml` in this repo for a commented example:

```toml
[defaults]
timeout_ms = 10000
interval_ms = 60000
# retention_days defaults to 90 when unset; set explicitly to keep more/less history

[web]
bind = "0.0.0.0:3000"

[database]
path = "./data/rusty-pingus.db"

[monitors]
path = "./monitors.toml"
```

### monitors.toml

Monitor definitions are an array of `[[monitors]]` tables, each tagged by `protocol`. This file is managed by the web UI (additions/removals are written back automatically), or you can edit it directly. Every monitor accepts two optional fields: `enabled = false` pauses it (kept in config and history, but not probed), and `retention_hours` overrides how long its data is kept (default: the global `retention_days × 24`).

```toml
# HTTP monitor
[[monitors]]
protocol = "http"
name = "my-api"
url = "https://api.example.com/health"
interval_ms = 30000
expected_status = 200

# TCP monitor
[[monitors]]
protocol = "tcp"
name = "my-db"
host = "db.example.com"
port = 5432
interval_ms = 60000

# ICMP monitor (requires elevated privileges — see below)
[[monitors]]
protocol = "icmp"
name = "gateway"
host = "192.168.1.1"
interval_ms = 30000
count = 3  # echo requests per cycle; up if any reply (default 3, min 1)

# Public-IP monitor — tracks your external IP and flags changes
[[monitors]]
protocol = "publicip"
name = "my-public-ip"
interval_ms = 300000
# url = "https://checkip.amazonaws.com"  # optional; default has a built-in fallback

# Border monitor — localizes LAN vs ISP faults (requires ICMP privileges)
[[monitors]]
protocol = "border"
name = "home-border"
interval_ms = 30000
# gateway = "192.168.1.1"      # optional; auto-detected when omitted
# isp_gateway = "74.0.0.1"     # optional; auto-detected (first public hop)
upstream = "1.1.1.1"           # internet reference (default 1.1.1.1)

# Traceroute monitor — per-hop path latency (requires ICMP privileges)
[[monitors]]
protocol = "traceroute"
name = "path-to-cloudflare"
host = "1.1.1.1"
interval_ms = 5000             # minimum 500 ms enforced for this type
timeout_ms = 1000              # per-hop reply wait
max_hops = 30                  # optional (default 30)
queries_per_hop = 3            # optional (default 3)
retention_hours = 24           # optional (default 24 h)
```

### Configuration Reference

| Field | Type | Default | Description |
|---|---|---|---|
| `[defaults].timeout_ms` | integer | 10000 | Default probe timeout (ms) |
| `[defaults].interval_ms` | integer | 60000 | Default probe interval (ms) |
| `[defaults].retention_days` | integer | 90 | Delete results older than N days. Defaults to 90 when unset (probe history is pruned automatically so the database doesn't grow without bound — important at low poll intervals). Set a larger value to keep more history. |
| `[web].bind` | string | `0.0.0.0:3000` | Web server bind address |
| `[database].path` | string | `./data/rusty-pingus.db` | SQLite file path |
| `[monitors].path` | string | `./monitors.toml` | Path to the monitor definitions file |

Every monitor type also accepts two optional fields: `enabled` (boolean, default `true`; set `false` to pause without deleting) and `retention_hours` (integer; overrides the global retention for that monitor's data).

**HTTP monitor fields:**

| Field | Type | Required | Description |
|---|---|---|---|
| `name` | string | yes | Unique monitor name |
| `url` | string | yes | Target URL |
| `interval_ms` | integer | no | Probe interval in ms (overrides default) |
| `timeout_ms` | integer | no | Probe timeout in ms (overrides default) |
| `method` | string | no | HTTP method (default: `GET`) |
| `expected_status` | integer | no | Expected HTTP status (default: any 2xx) |
| `headers` | map | no | Request headers |
| `body` | string | no | Request body |

**TCP monitor fields:** `name`, `host`, `port`, `interval_ms`, `timeout_ms`

**ICMP monitor fields:** `name`, `host`, `interval_ms`, `timeout_ms`, `count`

The ICMP probe sends `count` echo requests per cycle (default 3, minimum 1), each bounded by `timeout_ms`. The monitor is reported **up if at least one** reply is received and **down only when all** echoes are lost; the recorded response time is the best (lowest) RTT among the replies. Packet loss is surfaced in the failure reason — a partial loss is noted while the monitor stays up (`partial_loss (1/3 replies)`), and a full loss reports the counts (`no_reply (0/3 replies)`). Set `count = 1` to reproduce single-shot behavior. This makes ICMP monitors resilient to the incidental packet loss common on lossy or VPN-tunneled links.

**Public-IP monitor fields:** `name`, `interval_ms`, `timeout_ms`, `url` (optional)

The public-IP monitor (`protocol = "publicip"`) periodically GETs an IP-echo service and records your host's external IP. `url` is optional: when omitted it queries `https://checkip.amazonaws.com` and falls back to `https://icanhazip.com` if that fails. The monitor is **up** when a service returns a parseable IP address — which is shown as the probe's *detail* on the dashboard — and **down** when no IP can be obtained. When the IP differs from the previously recorded one, the change is surfaced in the detail (e.g. `203.0.113.7 (changed from 198.51.100.4)`) so it's visible in history. Useful for spotting ISP IP rotations that affect DNS, port-forwarding, or allow-lists.

**Border monitor fields:** `name`, `interval_ms`, `timeout_ms`, `gateway` (optional), `isp_gateway` (optional), `upstream` (default `1.1.1.1`)

The border monitor (`protocol = "border"`) localizes a connectivity fault — *"is it me or them?"* — by ICMP-pinging your local gateway **and** an upstream reference each cycle, then classifying the result (carried in the probe's *detail*):

| Gateway | Upstream | Status | Detail |
|---|---|---|---|
| reachable | reachable | `up` | `ok` (with both RTTs) |
| reachable | unreachable | `down` | `isp_down` — gateway up, internet down |
| unreachable | (either) | `down` | `lan_down` — local gateway/LAN down |

Overall status follows **upstream** reachability and the recorded response time is the upstream RTT. `gateway` (your local egress gateway) and `isp_gateway` (the first public hop) are auto-detected via traceroute when omitted (best-effort on Linux/macOS/Windows); if the gateway can neither be configured nor detected, the monitor reports `down` with a clear reason rather than guessing — configuring `gateway` explicitly is the reliable path. Like ICMP monitors, the border monitor needs raw-socket privileges (see **ICMP Privileges** below).

**Traceroute monitor fields:** `name`, `host`, `interval_ms`, `timeout_ms`, `max_hops` (default 30), `queries_per_hop` (default 3), `retention_hours` (default 24)

The traceroute monitor (`protocol = "traceroute"`) records the full per-hop path to `host` each cycle — sending `queries_per_hop` ICMP echoes per hop (up to `max_hops`) and storing the min/avg/max RTT for every responding hop. The monitor detail page renders this as a per-hop latency view so you can see *where* along the path latency or loss appears, not just the endpoint result. Because a run is expensive and high-volume, the scheduling interval is clamped to a **500 ms minimum** and per-hop data is retained for `retention_hours` (default 24). Like ICMP and border monitors, it needs raw-socket privileges (see **ICMP Privileges** below).

## Running

```bash
# Basic
./rusty-pingus

# Custom config
./rusty-pingus --config /etc/rusty-pingus/config.toml

# Override bind/db path
./rusty-pingus --bind 127.0.0.1:8080 --db /var/lib/rusty-pingus/db.sqlite

# Override the monitors file location (default: ./monitors.toml)
./rusty-pingus --monitors /etc/rusty-pingus/monitors.toml

# Set log level
RUST_LOG=debug ./rusty-pingus
```

The `--monitors` flag overrides the monitors file path from `config.toml`. Monitors added or removed via the web UI are written back to this file automatically.

## Windows

### System tray
Double-click `rusty-pingus.exe` from Explorer or your Downloads folder. A cyan icon appears in the system tray (notification area, bottom-right). No console window opens.

- **Left-click** the tray icon → opens the dashboard in your default browser
- **Right-click** → "Open Dashboard", "Reload Config", or "Quit"
- On **first launch** (no existing database), the dashboard opens automatically

**Reload Config** re-reads `config.toml` from disk and applies it — primarily to pick up a changed `[web].bind` port — by relaunching the app, so there's no need to manually quit and restart. The web server briefly restarts on the new port; monitoring resumes automatically.

### Logs
Release builds write logs to `<data_dir>/logs/rusty-pingus.YYYY-MM-DD.log` (default: `./data/logs/`). For debug output, run from a terminal using a debug build (`cargo run`).

### First run / missing config
On first run, rusty-pingus generates default `config.toml` and `monitors.toml` files and starts with zero monitors. Add monitors via the web UI, or edit `monitors.toml` directly.

## ICMP Privileges

ICMP probing requires raw socket access. On Linux:

```bash
# Option 1: Run as root
sudo ./rusty-pingus

# Option 2: Grant capability to the binary
sudo setcap cap_net_raw+ep ./rusty-pingus
```

TCP and HTTP probes work without elevated privileges.

## API

- `GET /api/monitors` — current status of all monitors
- `GET /api/monitors/stream` — Server-Sent Events stream of live status updates (one event per probe completion: name, status, response time, failure reason, last-checked time). The dashboard subscribes to this for near-real-time updates and falls back to polling `/api/monitors` if it's unavailable.
- `GET /api/monitors/:name/history?from=<iso8601>&to=<iso8601>&limit=100` — raw probe history
- `GET /api/monitors/:name/uptime` — uptime % for 1h, 24h, 7d, 30d windows
- `GET /api/monitors/:name/series?from=<iso8601>&to=<iso8601>&buckets=300` — response time aggregated into a bounded number of time buckets (each: bucket start, avg/min/max ms, sample count, up-ratio). Used by the monitor detail chart so any window stays fast regardless of poll interval. `buckets` is clamped to 50–1000; defaults are the last 24h with ~300 buckets.
- `GET /api/monitors/:name/segments?from=<iso8601>&to=<iso8601>` — contiguous up/down segments over the window, for drawing downtime bands.
- `GET /api/monitors/:name/extent` — the earliest and latest retained timestamps for the monitor (the bounds available to chart/history queries).
- `GET /api/monitors/:name/traceroute?from=<iso8601>&to=<iso8601>` — per-hop traceroute data (traceroute monitors only).
- `GET /api/monitors/:name/traceroute/extent` — the retained time range of traceroute data for the monitor.

**Monitor management** (these mutate `monitors.toml`):

- `GET /api/monitors/config` — full monitor definitions (the parsed `monitors.toml`).
- `POST /api/monitors` — add a monitor.
- `POST /api/monitors/:name/enabled` — enable/disable a monitor (pauses probing without deleting it).
- `DELETE /api/monitors/:name` — remove a monitor.

Wide chart windows and long uptime windows (7d/30d) are served from **per-minute rollups** (a background task aggregates `probe_results` into a `probe_rollup_1m` table), so their cost scales with minutes rather than the raw row count — important at low poll intervals. Fine/recent ranges still read raw results, and queries fall back to raw until the rollup has backfilled.

## systemd

```ini
[Unit]
Description=Rusty Pingus Uptime Monitor
After=network.target

[Service]
ExecStart=/usr/local/bin/rusty-pingus --config /etc/rusty-pingus/config.toml
Restart=on-failure
WorkingDirectory=/var/lib/rusty-pingus

[Install]
WantedBy=multi-user.target
```

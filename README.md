# rusty-pingus

Self-hosted uptime monitor written in Rust. Monitors external endpoints via HTTP, TCP, ICMP, public-IP, and border checks from your own network location. Stores results in SQLite and serves a web dashboard.

## Features

- HTTP(S), TCP, ICMP, public-IP, and border probing
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

Create a `config.toml` (see `config.toml` in this repo for a full example):

```toml
[defaults]
timeout_ms = 10000
interval_ms = 60000
# retention_days defaults to 90 when unset; set explicitly to keep more/less history

[web]
bind = "0.0.0.0:3000"

[database]
path = "./data/rusty-pingus.db"

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
# gateway = "192.168.1.1"  # optional; auto-detected when omitted
upstream = "1.1.1.1"        # upstream reference (default 1.1.1.1)
```

### Configuration Reference

| Field | Type | Default | Description |
|---|---|---|---|
| `[defaults].timeout_ms` | integer | 10000 | Default probe timeout (ms) |
| `[defaults].interval_ms` | integer | 60000 | Default probe interval (ms) |
| `[defaults].retention_days` | integer | 90 | Delete results older than N days. Defaults to 90 when unset (probe history is pruned automatically so the database doesn't grow without bound — important at low poll intervals). Set a larger value to keep more history. |
| `[web].bind` | string | `0.0.0.0:3000` | Web server bind address |
| `[database].path` | string | `./data/rusty-pingus.db` | SQLite file path |

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

**Border monitor fields:** `name`, `interval_ms`, `timeout_ms`, `gateway` (optional), `upstream` (default `1.1.1.1`)

The border monitor (`protocol = "border"`) localizes a connectivity fault — *"is it me or them?"* — by ICMP-pinging your local gateway **and** an upstream reference each cycle, then classifying the result (carried in the probe's *detail*):

| Gateway | Upstream | Status | Detail |
|---|---|---|---|
| reachable | reachable | `up` | `ok` (with both RTTs) |
| reachable | unreachable | `down` | `isp_down` — gateway up, internet down |
| unreachable | (either) | `down` | `lan_down` — local gateway/LAN down |

Overall status follows **upstream** reachability and the recorded response time is the upstream RTT. `gateway` is auto-detected from the system routing table when omitted (best-effort on Linux/macOS/Windows); if it can neither be configured nor detected, the monitor reports `down` with a clear reason rather than guessing — configuring `gateway` explicitly is the reliable path. Like ICMP monitors, the border monitor needs raw-socket privileges (see **ICMP Privileges** below).

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
- **Right-click** → "Open Dashboard" or "Quit"
- On **first launch** (no existing database), the dashboard opens automatically

### Logs
Release builds write logs to `<data_dir>/logs/rusty-pingus.YYYY-MM-DD.log` (default: `./data/logs/`). For debug output, run from a terminal using a debug build (`cargo run`).

### First run / missing config
If no `config.toml` is present, rusty-pingus generates a default one and starts with zero monitors. Edit the generated file and restart to add monitors.

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

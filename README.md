# rusty-pingus

Self-hosted uptime monitor written in Rust. Monitors external endpoints via HTTP, TCP, and ICMP from your own network location. Stores results in SQLite and serves a web dashboard.

## Features

- HTTP(S), TCP, and ICMP probing
- Per-monitor configurable intervals and timeouts
- SQLite persistence with automatic migrations
- Web dashboard at `http://localhost:3000`
- JSON API for programmatic access
- Single self-contained binary
- **Windows**: system tray icon, double-click to open dashboard, no console window
- Auto-generates a default `config.toml` on first run

## Installation

```bash
cargo build --release
cp target/release/rusty-pingus /usr/local/bin/
```

## Configuration

Create a `config.toml` (see `config.toml` in this repo for a full example):

```toml
[defaults]
timeout_secs = 10
interval_secs = 60
# retention_days = 90   # Uncomment to prune results older than N days

[web]
bind = "0.0.0.0:3000"

[database]
path = "./data/rusty-pingus.db"

# HTTP monitor
[[monitors]]
protocol = "http"
name = "my-api"
url = "https://api.example.com/health"
interval_secs = 30
expected_status = 200

# TCP monitor
[[monitors]]
protocol = "tcp"
name = "my-db"
host = "db.example.com"
port = 5432
interval_secs = 60

# ICMP monitor (requires elevated privileges — see below)
[[monitors]]
protocol = "icmp"
name = "gateway"
host = "192.168.1.1"
interval_secs = 30
```

### Configuration Reference

| Field | Type | Default | Description |
|---|---|---|---|
| `[defaults].timeout_secs` | integer | 10 | Default probe timeout |
| `[defaults].interval_secs` | integer | 60 | Default probe interval |
| `[defaults].retention_days` | integer | — | Delete results older than N days |
| `[web].bind` | string | `0.0.0.0:3000` | Web server bind address |
| `[database].path` | string | `./data/rusty-pingus.db` | SQLite file path |

**HTTP monitor fields:**

| Field | Type | Required | Description |
|---|---|---|---|
| `name` | string | yes | Unique monitor name |
| `url` | string | yes | Target URL |
| `interval_secs` | integer | no | Probe interval (overrides default) |
| `timeout_secs` | integer | no | Probe timeout (overrides default) |
| `method` | string | no | HTTP method (default: `GET`) |
| `expected_status` | integer | no | Expected HTTP status (default: any 2xx) |
| `headers` | map | no | Request headers |
| `body` | string | no | Request body |

**TCP monitor fields:** `name`, `host`, `port`, `interval_secs`, `timeout_secs`

**ICMP monitor fields:** `name`, `host`, `interval_secs`, `timeout_secs`

## Running

```bash
# Basic
./rusty-pingus

# Custom config
./rusty-pingus --config /etc/rusty-pingus/config.toml

# Override bind/db path
./rusty-pingus --bind 127.0.0.1:8080 --db /var/lib/rusty-pingus/db.sqlite

# Set log level
RUST_LOG=debug ./rusty-pingus
```

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
- `GET /api/monitors/:name/history?from=<iso8601>&to=<iso8601>&limit=100` — probe history
- `GET /api/monitors/:name/uptime` — uptime % for 1h, 24h, 7d, 30d windows

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

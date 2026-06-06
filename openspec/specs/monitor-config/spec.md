## Purpose

Defines how monitors are configured via a TOML file at startup, including supported protocols (HTTP, TCP, ICMP), per-monitor settings, and global defaults with per-monitor overrides.

## Requirements

### Requirement: Monitor definition via TOML config file
The system SHALL read monitor definitions from `monitors.toml` at startup. `config.toml` SHALL contain only application settings (`[web]`, `[database]`, `[defaults]`) with no `[[monitors]]` entries. If `monitors.toml` does not exist at the configured path, the system SHALL generate a default file at that path and proceed with zero monitors rather than exiting.

#### Scenario: Valid monitors.toml loaded at startup
- **WHEN** the binary starts with a valid `monitors.toml` present
- **THEN** all monitors defined in the file are registered and begin probing

#### Scenario: Missing monitors.toml generates default and continues
- **WHEN** the binary starts and no `monitors.toml` is found at the configured path
- **THEN** a default `monitors.toml` is written to that path with commented examples, a warning is logged, and the binary starts with zero monitors

#### Scenario: Malformed monitors.toml
- **WHEN** `monitors.toml` contains invalid TOML or an unrecognised protocol field
- **THEN** the binary exits with a non-zero status code and reports which field or section is invalid

#### Scenario: config.toml contains no monitor entries
- **WHEN** `config.toml` is loaded
- **THEN** it is parsed for app settings only; no `[[monitors]]` entries are expected or accepted in config.toml

### Requirement: HTTP monitor configuration
The system SHALL support HTTP/HTTPS monitors with configurable method, expected status code range, timeout (`timeout_ms`), optional request headers, and optional request body.

#### Scenario: HTTP monitor with default settings
- **WHEN** an HTTP monitor is defined with only `url` and `interval_ms`
- **THEN** it defaults to GET method, 2xx expected status, and a 10000 ms (10 second) timeout

#### Scenario: HTTP monitor with custom headers
- **WHEN** an HTTP monitor defines `headers` as a key-value map
- **THEN** those headers are sent with every probe request

#### Scenario: HTTP monitor with expected status code
- **WHEN** an HTTP monitor defines `expected_status = 200`
- **THEN** any response with a different status code is recorded as a failure

### Requirement: TCP monitor configuration
The system SHALL support TCP monitors that probe a host:port for a successful connection within a timeout (`timeout_ms`).

#### Scenario: TCP monitor definition
- **WHEN** a TCP monitor is defined with `host`, `port`, and `interval_ms`
- **THEN** the monitor attempts a TCP connection on each probe cycle

#### Scenario: TCP monitor with custom timeout
- **WHEN** a TCP monitor defines `timeout_ms`
- **THEN** connections that exceed the timeout are recorded as failures

### Requirement: ICMP monitor configuration
The system SHALL support ICMP ping monitors with configurable interval (`interval_ms`), timeout (`timeout_ms`), and packet count (`count`). The `count` field SHALL set how many ICMP echo requests are sent per probe cycle. When `count` is omitted the system SHALL apply a default of 3; any configured value below 1 SHALL be treated as 1 so a cycle never sends zero packets.

#### Scenario: ICMP monitor definition
- **WHEN** an ICMP monitor is defined with `host` and `interval_ms`
- **THEN** the monitor sends ICMP echo requests on each probe cycle using the default packet count of 3

#### Scenario: ICMP monitor with custom packet count
- **WHEN** an ICMP monitor defines `count = 5`
- **THEN** each probe cycle sends 5 echo requests

#### Scenario: ICMP packet count floor
- **WHEN** an ICMP monitor defines `count = 0` (or a negative/absent value)
- **THEN** the effective packet count is clamped so at least one echo request is sent per cycle

#### Scenario: ICMP monitor without required privileges
- **WHEN** the process lacks CAP_NET_RAW or root privileges and an ICMP monitor is configured
- **THEN** the binary logs a clear warning at startup and marks the ICMP monitor as errored rather than crashing

### Requirement: Per-monitor check interval
The system SHALL allow each monitor to define its own check interval (`interval_ms`) independently of other monitors.

#### Scenario: Mixed intervals
- **WHEN** monitor A has `interval_ms = 30000` and monitor B has `interval_ms = 300000`
- **THEN** monitor A probes every 30 seconds and monitor B probes every 5 minutes, independently

### Requirement: Global defaults with per-monitor overrides
The system SHALL support a `[defaults]` section in the config file that sets default values for timeout (`timeout_ms`) and interval (`interval_ms`), which individual monitors can override.

#### Scenario: Monitor inherits default timeout
- **WHEN** `[defaults]` defines `timeout_ms = 10000` and a monitor does not specify a timeout
- **THEN** that monitor uses a 10000 ms timeout

#### Scenario: Monitor overrides default
- **WHEN** a monitor explicitly defines `timeout_ms = 5000`
- **THEN** that monitor uses 5000 ms regardless of the default

### Requirement: Millisecond timing units with legacy migration
The system SHALL express monitor check interval and timeout in milliseconds using the fields `interval_ms` and `timeout_ms`. When a monitor or `[defaults]` entry is loaded with the legacy second-based keys (`interval_secs`, `timeout_secs`), the system SHALL convert each legacy value to milliseconds by multiplying by 1000, log a warning describing the conversion, and rewrite the file in the new `_ms` form so the migration occurs only once. When both the `_ms` and `_secs` form of the same field are present, the `_ms` value SHALL take precedence.

#### Scenario: New file uses millisecond fields
- **WHEN** `monitors.toml` defines a monitor with `interval_ms = 60000` and `timeout_ms = 10000`
- **THEN** the monitor probes every 60000 ms with a 10000 ms timeout

#### Scenario: Legacy second-based values are migrated
- **WHEN** an existing `monitors.toml` defines `interval_secs = 60` and `timeout_secs = 10`
- **THEN** the values are loaded as `interval_ms = 60000` and `timeout_ms = 10000`, a warning is logged, and the file is rewritten using the `_ms` fields

#### Scenario: Millisecond field wins over legacy field
- **WHEN** a monitor entry contains both `interval_ms = 30000` and `interval_secs = 60`
- **THEN** the effective interval is 30000 ms

### Requirement: Traceroute monitor configuration
The system SHALL support a `traceroute` monitor type configured with a target host, a check interval, a per-hop timeout, a maximum hop count, a number of queries per hop, and a retention period. Configuration SHALL be accepted from the TOML monitors file and the web UI, consistent with the other monitor types.

#### Scenario: Traceroute monitor parsed from config
- **WHEN** the monitors file contains an entry with `protocol = "traceroute"` and a target host
- **THEN** the monitor loads with its target, interval, timeout, max hops, queries-per-hop, and retention applied (defaults filling any omitted optional fields)

#### Scenario: Sensible defaults for optional fields
- **WHEN** a traceroute monitor omits max hops, queries-per-hop, or retention
- **THEN** documented defaults are applied (e.g. max hops 30, queries-per-hop 3, a default retention)

### Requirement: Traceroute interval has an enforced 500 ms floor
A `traceroute` monitor's check interval SHALL be clamped to a minimum of 500 milliseconds. A configured interval below the floor SHALL be raised to 500 ms rather than rejected, applied uniformly to both TOML and UI configuration paths.

#### Scenario: Sub-floor interval is raised
- **WHEN** a traceroute monitor is configured with an interval below 500 ms
- **THEN** the effective interval is 500 ms

#### Scenario: At-or-above floor interval is preserved
- **WHEN** a traceroute monitor is configured with an interval of 500 ms or more
- **THEN** the configured interval is used unchanged

### Requirement: Monitors have an enabled flag
Every monitor SHALL have an `enabled` boolean that defaults to `true` when omitted, so existing configuration files without the key load as enabled. A monitor with `enabled = false` SHALL be retained in configuration (and keep its stored history) but treated as paused. The flag SHALL round-trip through the config file (a disabled monitor stays disabled across reloads).

#### Scenario: Missing flag defaults to enabled
- **WHEN** a monitor entry omits `enabled`
- **THEN** the monitor loads as enabled

#### Scenario: Disabled monitor is retained, not removed
- **WHEN** a monitor has `enabled = false`
- **THEN** it remains in the configuration and its stored history is preserved

#### Scenario: Disabled state persists across reload
- **WHEN** a monitor is disabled and the configuration is reloaded
- **THEN** the monitor is still disabled

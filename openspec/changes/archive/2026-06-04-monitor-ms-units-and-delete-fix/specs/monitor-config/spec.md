## ADDED Requirements

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

## MODIFIED Requirements

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

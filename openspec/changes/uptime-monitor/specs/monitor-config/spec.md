## ADDED Requirements

### Requirement: Monitor definition via TOML config file
The system SHALL read monitor definitions from a TOML configuration file at startup. Each monitor entry SHALL specify at minimum: a unique name, a target endpoint, a protocol, and a check interval.

#### Scenario: Valid config file loaded at startup
- **WHEN** the binary starts with a valid `config.toml` present
- **THEN** all monitors defined in the file are registered and begin probing

#### Scenario: Missing config file
- **WHEN** the binary starts and no config file is found at the configured path
- **THEN** the binary exits with a non-zero status code and a human-readable error message

#### Scenario: Malformed config file
- **WHEN** the config file contains invalid TOML or missing required fields
- **THEN** the binary exits with a non-zero status code and reports which field or section is invalid

### Requirement: HTTP monitor configuration
The system SHALL support HTTP/HTTPS monitors with configurable method, expected status code range, timeout, optional request headers, and optional request body.

#### Scenario: HTTP monitor with default settings
- **WHEN** an HTTP monitor is defined with only `url` and `interval`
- **THEN** it defaults to GET method, 2xx expected status, and 10-second timeout

#### Scenario: HTTP monitor with custom headers
- **WHEN** an HTTP monitor defines `headers` as a key-value map
- **THEN** those headers are sent with every probe request

#### Scenario: HTTP monitor with expected status code
- **WHEN** an HTTP monitor defines `expected_status = 200`
- **THEN** any response with a different status code is recorded as a failure

### Requirement: TCP monitor configuration
The system SHALL support TCP monitors that probe a host:port for a successful connection within a timeout.

#### Scenario: TCP monitor definition
- **WHEN** a TCP monitor is defined with `host`, `port`, and `interval`
- **THEN** the monitor attempts a TCP connection on each probe cycle

#### Scenario: TCP monitor with custom timeout
- **WHEN** a TCP monitor defines `timeout_secs`
- **THEN** connections that exceed the timeout are recorded as failures

### Requirement: ICMP monitor configuration
The system SHALL support ICMP ping monitors with configurable interval, timeout, and packet count.

#### Scenario: ICMP monitor definition
- **WHEN** an ICMP monitor is defined with `host` and `interval`
- **THEN** the monitor sends ICMP echo requests on each probe cycle

#### Scenario: ICMP monitor without required privileges
- **WHEN** the process lacks CAP_NET_RAW or root privileges and an ICMP monitor is configured
- **THEN** the binary logs a clear warning at startup and marks the ICMP monitor as errored rather than crashing

### Requirement: Per-monitor check interval
The system SHALL allow each monitor to define its own check interval independently of other monitors.

#### Scenario: Mixed intervals
- **WHEN** monitor A has `interval_secs = 30` and monitor B has `interval_secs = 300`
- **THEN** monitor A probes every 30 seconds and monitor B probes every 5 minutes, independently

### Requirement: Global defaults with per-monitor overrides
The system SHALL support a `[defaults]` section in the config file that sets default values for timeout and interval, which individual monitors can override.

#### Scenario: Monitor inherits default timeout
- **WHEN** `[defaults]` defines `timeout_secs = 10` and a monitor does not specify a timeout
- **THEN** that monitor uses a 10-second timeout

#### Scenario: Monitor overrides default
- **WHEN** a monitor explicitly defines `timeout_secs = 5`
- **THEN** that monitor uses 5 seconds regardless of the default

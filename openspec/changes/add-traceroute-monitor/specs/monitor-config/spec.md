## ADDED Requirements

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

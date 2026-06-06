## MODIFIED Requirements

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

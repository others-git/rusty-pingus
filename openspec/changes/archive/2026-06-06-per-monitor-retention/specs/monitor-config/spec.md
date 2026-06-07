## ADDED Requirements

### Requirement: Per-monitor retention period
Every monitor SHALL support a `retention_hours` setting controlling how long that monitor's stored data is kept. When omitted, a monitor SHALL fall back to the system's global default retention (so existing configurations are unaffected). For a traceroute monitor, a legacy `retention_ms` value SHALL be accepted and converted to hours, and the resulting per-monitor retention SHALL govern all of that monitor's stored data.

#### Scenario: Retention configured in hours
- **WHEN** a monitor sets `retention_hours = 6`
- **THEN** that monitor's data is retained for 6 hours

#### Scenario: Omitted retention falls back to the global default
- **WHEN** a monitor does not specify `retention_hours`
- **THEN** the global default retention applies to that monitor (existing behavior)

#### Scenario: Legacy traceroute retention is accepted
- **WHEN** a traceroute monitor specifies the legacy `retention_ms`
- **THEN** it is interpreted as that monitor's retention (converted to hours) and governs its stored data

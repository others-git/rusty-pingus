## ADDED Requirements

### Requirement: Compact probe-result storage representation
The system SHALL store probe results compactly: probe timestamps SHALL be persisted as an integer epoch (not a text datetime), and fields that are constant for a monitor (its protocol and endpoint) SHALL NOT be duplicated on every probe row. This representation is internal; the persistence, history, current-status, and series query contracts — including timestamps returned as RFC3339 strings and the protocol/endpoint fields present in responses — SHALL be preserved unchanged.

#### Scenario: Stored timestamps are integer epochs but returned as RFC3339
- **WHEN** a probe result is stored and later read via the history or current-status query
- **THEN** the timestamp is persisted as an integer epoch and the query returns it as the same RFC3339 string a client received before this change

#### Scenario: Per-monitor-constant fields are not duplicated per row
- **WHEN** probe results are stored for a monitor
- **THEN** the monitor's protocol and endpoint are not written on every row, yet history/current-status/list responses still include the monitor's protocol and endpoint

#### Scenario: Existing data is migrated, not lost
- **WHEN** the application starts against a database created before this change
- **THEN** existing probe rows are re-encoded to the compact representation and remain queryable with identical results

### Requirement: Dashboard status list served without per-monitor query fan-out
The endpoint backing the dashboard's monitor list SHALL assemble each monitor's latest status and 24-hour uptime using a fixed, small number of queries rather than a number of queries that grows with the monitor count. The response content SHALL be unchanged.

#### Scenario: Listing cost does not scale with monitor count
- **WHEN** the dashboard list endpoint is called with many configured monitors
- **THEN** it returns the same per-monitor status and 24h uptime as before, without issuing per-monitor query fan-out

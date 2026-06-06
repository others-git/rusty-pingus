## ADDED Requirements

### Requirement: Compact traceroute hop storage
The system SHALL persist traceroute results in dedicated storage separate from `probe_results`, designed to minimize database size for high-volume, route-stable data. Distinct hop addresses SHALL be interned so each address is stored once and referenced by id, and round-trip times SHALL be stored as integer microseconds. A non-responding hop SHALL be representable without an address or RTT.

#### Scenario: Repeated hop addresses are interned, not duplicated
- **WHEN** successive runs traverse the same router addresses
- **THEN** each distinct address is stored once and referenced by the hops of every run, rather than repeated per run

#### Scenario: Non-responding hop stored compactly
- **WHEN** a run includes a hop that did not respond
- **THEN** the hop is persisted with no address and no RTT, marked as lost

### Requirement: Per-monitor traceroute retention with pruning
Each `traceroute` monitor SHALL have a configurable retention period, and the system SHALL periodically delete traceroute runs (and their hops) older than the monitor's retention, then release interned addresses that are no longer referenced. This bounds database growth for the type without relying on fixed rollup windows.

#### Scenario: Runs older than retention are pruned
- **WHEN** the maintenance task runs and a monitor has runs older than its retention period
- **THEN** those runs and their hops are deleted

#### Scenario: Orphaned addresses are reclaimed
- **WHEN** pruning removes the last run referencing an interned address
- **THEN** that address row is removed so it does not accumulate

#### Scenario: Within-retention data is preserved
- **WHEN** the maintenance task runs
- **THEN** runs newer than the retention cutoff are kept intact

### Requirement: Traceroute range query
The system SHALL provide a query returning a monitor's traceroute hops aggregated over an explicit `[from, to]` time range, suitable for driving a time-range-refining UI control over the retained data.

#### Scenario: Hops aggregated over a time range
- **WHEN** the query is invoked with a from/to range
- **THEN** it returns per-hop-position aggregates (address, reachability, min/avg/max RTT) over the runs whose timestamps fall in the range

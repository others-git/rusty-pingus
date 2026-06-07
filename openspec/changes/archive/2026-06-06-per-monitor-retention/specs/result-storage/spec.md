## ADDED Requirements

### Requirement: Per-monitor retention pruning
The system SHALL prune each monitor's stored data according to that monitor's own retention period rather than a single global value. A periodic cleanup SHALL delete a monitor's `probe_results` and minute rollups older than its retention, and (for traceroute monitors) its hop data older than the same retention. A monitor without an explicit retention SHALL be pruned at the global default. Pruning SHALL pick up retention changes without an application restart.

#### Scenario: Each monitor pruned by its own retention
- **WHEN** monitor A has 6-hour retention and monitor B has 30-day retention
- **THEN** the cleanup deletes A's results/rollups older than 6 hours and B's older than 30 days

#### Scenario: Unconfigured monitor uses the global default
- **WHEN** a monitor has no explicit retention
- **THEN** its data is pruned at the global default retention

#### Scenario: Retention change applies without restart
- **WHEN** a monitor's retention is shortened while the app is running
- **THEN** the next cleanup prunes that monitor's data to the new retention without a restart

### Requirement: State-timeline range query is not capped to newest rows
The system SHALL provide state-timeline data (collapsed runs of the same state — e.g. public IP or border fault class — with start/end) for a monitor over an explicit `[from, to]` range, covering the entire range rather than only the most recent N raw probes. The result SHALL be bounded by the number of state changes, not the number of probes.

#### Scenario: Whole window represented regardless of probe frequency
- **WHEN** a high-frequency state-timeline monitor has far more than 1000 probes in the requested range
- **THEN** the returned segments still span the entire range (older state changes are not dropped)

#### Scenario: Bounded by changes, not probes
- **WHEN** a monitor's state rarely changes over a large range
- **THEN** the response contains only the few segments for those changes, not one entry per probe

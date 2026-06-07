## Purpose

Defines how probe results are persisted to and queried from a SQLite database, including schema initialization, result storage, history and uptime queries, and result retention policies.

## Requirements

### Requirement: SQLite database initialization
The system SHALL automatically create and migrate the SQLite database schema on startup if the database file does not exist or is not up to date.

#### Scenario: First run creates database
- **WHEN** the binary starts and no database file exists at the configured path
- **THEN** a new SQLite database is created and all schema migrations are applied before probing begins

#### Scenario: Existing database is up to date
- **WHEN** the binary starts and the database schema is already at the latest version
- **THEN** no migrations are applied and startup proceeds normally

### Requirement: Probe result persistence
The system SHALL persist every probe result to the SQLite database immediately after the probe completes.

#### Scenario: Successful probe result stored
- **WHEN** a probe completes with status `up`
- **THEN** a row is inserted into the `probe_results` table with the correct monitor name, timestamp, status, and response time

#### Scenario: Failed probe result stored
- **WHEN** a probe completes with status `down`
- **THEN** a row is inserted into the `probe_results` table with the correct monitor name, timestamp, status, and failure reason

#### Scenario: Database write failure does not crash probing
- **WHEN** a database write fails (e.g., disk full)
- **THEN** the error is logged and the probe task continues running; subsequent probes are still attempted

### Requirement: Current monitor status query
The system SHALL provide a query to retrieve the current status of all monitors (most recent probe result per monitor).

#### Scenario: Current status reflects latest probe
- **WHEN** a monitor has completed multiple probes
- **THEN** the current status query returns only the most recent result for that monitor

### Requirement: Probe history query
The system SHALL provide a query to retrieve the probe result history for a specific monitor within a given time range, ordered by timestamp descending.

#### Scenario: History query with time range
- **WHEN** a history query is issued for monitor `my-api` with a `from` and `to` timestamp
- **THEN** only results within that range are returned, ordered newest first

### Requirement: Uptime percentage calculation
The system SHALL provide a query that calculates the uptime percentage for a monitor over a configurable rolling window (e.g., last 24h, 7d, 30d). For long windows the calculation MAY be served from the minute rollups (summing up counts and total counts) for bounded cost on large datasets; for short windows it MAY use raw results. The result SHALL be equivalent regardless of source.

#### Scenario: Uptime percentage over 24 hours
- **WHEN** an uptime query is issued for monitor `my-api` with `window = 24h`
- **THEN** the result is `(count of 'up' results / total results in window) * 100`, rounded to two decimal places

#### Scenario: Long-window uptime is bounded
- **WHEN** an uptime query is issued for a 30-day window over a monitor with millions of raw results
- **THEN** the query reads minute rollups so its cost scales with minutes, not raw rows

#### Scenario: No results in window
- **WHEN** an uptime query is issued for a monitor with no results in the specified window
- **THEN** the result is `null` (not 0%), indicating insufficient data

### Requirement: Aggregated probe series query
The system SHALL provide a query that aggregates a monitor's probe results over a `[from, to]` time range into a bounded number of time buckets. Each bucket SHALL report the bucket start time, average/min/max response time, sample count, and the fraction of `up` results (up-ratio). The number of buckets returned SHALL NOT exceed the requested target, regardless of how many raw results fall in the range. When each output bucket spans at least one minute, the query SHALL be served from the minute rollups for bounded cost; for finer ranges it SHALL use raw results.

#### Scenario: Results bucketed over a range
- **WHEN** an aggregated series is requested for a monitor over a `[from, to]` range with a target bucket count N
- **THEN** results are grouped into at most N time buckets, each reporting bucket start, avg/min/max response time, sample count, and up-ratio

#### Scenario: Wide range served from rollups
- **WHEN** an aggregated series is requested over a wide range where each bucket spans at least one minute (e.g. a 30-day window)
- **THEN** the query reads minute rollups so its cost scales with minutes, not raw rows, and still returns at most N buckets

#### Scenario: Fine range served from raw
- **WHEN** an aggregated series is requested over a range narrow enough that buckets span less than a minute
- **THEN** the query reads raw results so sub-minute detail is preserved

#### Scenario: Empty range
- **WHEN** no results exist in the requested range
- **THEN** the query returns an empty set

### Requirement: Result retention policy
The system SHALL support a configurable retention period. Probe results older than the retention period SHALL be automatically purged, and minute rollups older than the retention period SHALL be purged as well. When no retention period is configured, the system SHALL apply a default retention of 90 days rather than retaining results indefinitely. An operator MAY set an explicit retention period (including a larger value) to override the default.

#### Scenario: Old results pruned
- **WHEN** the retention period is set to `90d` and results older than 90 days exist
- **THEN** those raw results and their minute rollups are deleted from the database during a periodic cleanup task

#### Scenario: Default retention applied when unconfigured
- **WHEN** no retention period is configured
- **THEN** the system prunes raw results and rollups older than the default retention (90 days) during the periodic cleanup task

#### Scenario: Explicit override
- **WHEN** an operator sets an explicit retention period
- **THEN** that value is used instead of the default

### Requirement: Delete stored results for a monitor
The system SHALL provide a way to delete all stored probe results for a named monitor, including its minute rollups. This is invoked when a monitor is removed so that the monitor no longer appears in status queries derived from `probe_results` or in rollup-backed queries.

#### Scenario: Results deleted for a monitor
- **WHEN** a delete-results operation is issued for monitor `my-api`
- **THEN** all rows in `probe_results` and `probe_rollup_1m` with `monitor_name = 'my-api'` are removed

#### Scenario: Delete results for monitor with no history
- **WHEN** a delete-results operation is issued for a monitor that has no stored results
- **THEN** the operation succeeds and reports zero rows deleted

#### Scenario: Current status excludes a purged monitor
- **WHEN** a monitor's results have been deleted
- **THEN** the current monitor status query no longer returns that monitor

### Requirement: Minute-resolution probe rollups
The system SHALL maintain a per-minute rollup of probe results per monitor in a dedicated table. For each monitor and each one-minute bucket, the rollup SHALL store the total probe count, the count of `up` probes, the sum of response times over `up` probes, and the min and max response time over `up` probes. These fields SHALL allow exact reconstruction of count, up-ratio, min, and max for any coarser bucketing, and the average response time over successful probes.

#### Scenario: Minute aggregates stored
- **WHEN** probes for a monitor complete within a given minute
- **THEN** a rollup row for that monitor and minute records the probe count, up count, summed up response time, and min/max up response time

#### Scenario: Re-bucketing from rollups is exact for counts and extremes
- **WHEN** several minute rollups are combined into a coarser bucket
- **THEN** the combined count, up-ratio, min, and max equal what the same computation over the underlying raw results would produce, and the average equals summed up response time divided by up count

### Requirement: Rollup maintenance
The system SHALL keep the minute rollups current via a background process that aggregates newly completed minutes from the raw results. On startup it SHALL backfill any missing range (including all existing history when the rollup is empty, and any gap caused by downtime). The in-progress current minute SHALL NOT be rolled up until complete.

#### Scenario: New minutes rolled up incrementally
- **WHEN** a minute completes and the maintenance process next runs
- **THEN** a rollup row for that minute is created (or replaced) from the raw results of that minute

#### Scenario: Backfill on startup
- **WHEN** the binary starts and the rollup is missing minutes that exist in raw results (empty rollup, or a gap from downtime)
- **THEN** the maintenance process aggregates those minutes into the rollup without blocking startup

#### Scenario: Current minute excluded
- **WHEN** the maintenance process runs partway through a minute
- **THEN** that in-progress minute is not yet rolled up; it is rolled up once complete

### Requirement: Per-probe detail context
The system SHALL store an optional, human-readable detail string with each probe result, capturing type-specific context (for example the observed public IP, or a border monitor's fault localization). The field SHALL be nullable; monitor types that have no extra context SHALL leave it unset. Stored detail SHALL be returned with probe history and current-status queries.

#### Scenario: Detail persisted and returned
- **WHEN** a probe result carries a detail value
- **THEN** that value is stored and returned in the monitor's history and current status

#### Scenario: No detail for plain checks
- **WHEN** a probe type records no extra context (e.g. a basic TCP check)
- **THEN** the detail is null and nothing extra is shown

#### Scenario: Existing results unaffected
- **WHEN** the detail field is added to existing stored results
- **THEN** prior results remain valid with a null detail

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

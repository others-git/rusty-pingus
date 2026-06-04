## ADDED Requirements

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

## MODIFIED Requirements

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

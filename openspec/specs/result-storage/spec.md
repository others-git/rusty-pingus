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
The system SHALL provide a query that calculates the uptime percentage for a monitor over a configurable rolling window (e.g., last 24h, 7d, 30d).

#### Scenario: Uptime percentage over 24 hours
- **WHEN** an uptime query is issued for monitor `my-api` with `window = 24h`
- **THEN** the result is `(count of 'up' results / total results in window) * 100`, rounded to two decimal places

#### Scenario: No results in window
- **WHEN** an uptime query is issued for a monitor with no results in the specified window
- **THEN** the result is `null` (not 0%), indicating insufficient data

### Requirement: Result retention policy
The system SHALL support a configurable retention period. Probe results older than the retention period SHALL be automatically purged.

#### Scenario: Old results pruned
- **WHEN** the retention period is set to `90d` and results older than 90 days exist
- **THEN** those results are deleted from the database during a periodic cleanup task

#### Scenario: Default retention period
- **WHEN** no retention period is configured
- **THEN** the system retains all results indefinitely

### Requirement: Delete stored results for a monitor
The system SHALL provide a way to delete all stored probe results for a named monitor. This is invoked when a monitor is removed so that the monitor no longer appears in status queries derived from `probe_results`.

#### Scenario: Results deleted for a monitor
- **WHEN** a delete-results operation is issued for monitor `my-api`
- **THEN** all rows in `probe_results` with `monitor_name = 'my-api'` are removed and the count of deleted rows is returned

#### Scenario: Delete results for monitor with no history
- **WHEN** a delete-results operation is issued for a monitor that has no stored results
- **THEN** the operation succeeds and reports zero rows deleted

#### Scenario: Current status excludes a purged monitor
- **WHEN** a monitor's results have been deleted
- **THEN** the current monitor status query no longer returns that monitor

## ADDED Requirements

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

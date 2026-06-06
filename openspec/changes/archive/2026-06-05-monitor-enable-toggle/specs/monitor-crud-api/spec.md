## ADDED Requirements

### Requirement: Toggle a monitor's enabled state via API
The system SHALL expose an endpoint to set a monitor's enabled state (enable or disable it by name) without deleting the monitor or its history. On success it SHALL persist the change, take effect on scheduling, and return the updated monitor list. Toggling a non-existent monitor SHALL return a not-found error.

#### Scenario: Disable a monitor
- **WHEN** the toggle endpoint is called for an existing monitor with enabled set to false
- **THEN** the monitor is marked disabled, persisted, and the updated list is returned

#### Scenario: Enable a monitor
- **WHEN** the toggle endpoint is called for an existing monitor with enabled set to true
- **THEN** the monitor is marked enabled, persisted, and the updated list is returned

#### Scenario: Toggle unknown monitor
- **WHEN** the toggle endpoint is called for a name that does not exist
- **THEN** a not-found error is returned and nothing is changed

### Requirement: Monitor list exposes enabled state
The monitor-list response SHALL include each monitor's `enabled` flag so clients can render enabled/disabled state without an extra request.

#### Scenario: enabled present in the list
- **WHEN** the monitor list is requested
- **THEN** each monitor entry includes its `enabled` boolean

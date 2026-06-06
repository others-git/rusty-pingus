## ADDED Requirements

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

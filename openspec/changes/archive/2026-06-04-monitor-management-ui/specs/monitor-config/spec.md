## MODIFIED Requirements

### Requirement: Monitor definition via TOML config file
The system SHALL read monitor definitions from `monitors.toml` at startup. `config.toml` SHALL contain only application settings (`[web]`, `[database]`, `[defaults]`) with no `[[monitors]]` entries. If `monitors.toml` does not exist at the configured path, the system SHALL generate a default file at that path and proceed with zero monitors rather than exiting.

#### Scenario: Valid monitors.toml loaded at startup
- **WHEN** the binary starts with a valid `monitors.toml` present
- **THEN** all monitors defined in the file are registered and begin probing

#### Scenario: Missing monitors.toml generates default and continues
- **WHEN** the binary starts and no `monitors.toml` is found at the configured path
- **THEN** a default `monitors.toml` is written to that path with commented examples, a warning is logged, and the binary starts with zero monitors

#### Scenario: Malformed monitors.toml
- **WHEN** `monitors.toml` contains invalid TOML or an unrecognised protocol field
- **THEN** the binary exits with a non-zero status code and reports which field or section is invalid

#### Scenario: config.toml contains no monitor entries
- **WHEN** `config.toml` is loaded
- **THEN** it is parsed for app settings only; no `[[monitors]]` entries are expected or accepted in config.toml

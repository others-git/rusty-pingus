## MODIFIED Requirements

### Requirement: Monitor definition via TOML config file
The system SHALL read monitor definitions from a TOML configuration file at startup. Each monitor entry SHALL specify at minimum: a unique name, a target endpoint, a protocol, and a check interval. If no config file exists at the configured path, the system SHALL generate a default config file at that path and proceed with zero monitors rather than exiting with an error.

#### Scenario: Valid config file loaded at startup
- **WHEN** the binary starts with a valid `config.toml` present
- **THEN** all monitors defined in the file are registered and begin probing

#### Scenario: Missing config file generates default and continues
- **WHEN** the binary starts and no config file is found at the configured path
- **THEN** a default `config.toml` is written to that path, a warning is logged, and the binary starts with zero monitors (no exit)

#### Scenario: Malformed config file
- **WHEN** the config file contains invalid TOML or missing required fields
- **THEN** the binary exits with a non-zero status code and reports which field or section is invalid

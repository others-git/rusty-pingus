## ADDED Requirements

### Requirement: Auto-generate default config on missing file
The system SHALL write a default `config.toml` to the configured path if the file does not exist at startup, then continue running with zero monitors configured.

#### Scenario: Missing config generates default file
- **WHEN** the binary starts and no config file exists at the configured path
- **THEN** a default `config.toml` is written to that path containing commented examples of each monitor type, and the binary proceeds with zero monitors

#### Scenario: Existing config is not overwritten
- **WHEN** a `config.toml` already exists at the configured path
- **THEN** the file is read as normal and no default is generated

#### Scenario: Default config is valid TOML
- **WHEN** the generated default `config.toml` is loaded by the binary
- **THEN** it parses without error and produces a config with zero monitors and default settings

### Requirement: User is informed about generated config
The system SHALL log a clear message (and show a tray notification on Windows) when a default config is generated, telling the user where the file was written.

#### Scenario: Log message on default generation
- **WHEN** a default config is generated
- **THEN** a log line at WARN level is emitted: "No config found at <path> — generated a default. Edit it to add monitors."

#### Scenario: Windows tray tooltip on default generation
- **WHEN** a default config is generated on Windows
- **THEN** the tray icon tooltip text includes "No config found — default generated at <path>"

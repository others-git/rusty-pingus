## ADDED Requirements

### Requirement: monitors.toml as monitor definition source
The system SHALL read monitor definitions exclusively from `monitors.toml` at runtime. The path SHALL be configurable (default: `./monitors.toml`). If the file does not exist, it SHALL be created with commented examples and the system SHALL start with zero monitors.

#### Scenario: monitors.toml loaded at startup
- **WHEN** the binary starts and a valid `monitors.toml` exists
- **THEN** all monitors defined in the file are registered with the scheduler

#### Scenario: missing monitors.toml generates default
- **WHEN** `monitors.toml` does not exist at the configured path
- **THEN** a default file is written with commented examples and the scheduler starts with zero monitors

#### Scenario: monitors.toml is separate from config.toml
- **WHEN** `config.toml` contains no `[[monitors]]` entries
- **THEN** `config.toml` is loaded for app settings and `monitors.toml` is loaded for monitor definitions independently

### Requirement: Migration of monitors from config.toml
The system SHALL detect `[[monitors]]` entries in `config.toml` on startup and automatically migrate them to `monitors.toml`, then remove them from `config.toml`.

#### Scenario: Monitors migrated on first run
- **WHEN** `config.toml` contains one or more `[[monitors]]` entries and `monitors.toml` does not yet exist
- **THEN** the monitors are written to `monitors.toml`, removed from `config.toml`, and a warning is logged explaining the migration

#### Scenario: No migration when already separated
- **WHEN** `config.toml` has no `[[monitors]]` entries
- **THEN** no migration occurs and `config.toml` is not modified

### Requirement: Thread-safe in-memory monitor store
The system SHALL maintain an in-memory list of monitors in a thread-safe store (`Arc<RwLock<...>>`) that is the single source of truth during runtime. The store SHALL support concurrent reads and serialized writes.

#### Scenario: Concurrent reads do not block each other
- **WHEN** multiple async tasks read the monitor list simultaneously
- **THEN** all reads complete without blocking each other

#### Scenario: Write serialization
- **WHEN** two concurrent API requests attempt to add a monitor simultaneously
- **THEN** one write completes fully before the other begins; no data corruption occurs

### Requirement: Persist store changes to monitors.toml
Every write operation (add, delete) on the monitor store SHALL immediately serialize the current in-memory monitor list to `monitors.toml`. The write SHALL be atomic (write to a temp file, then rename).

#### Scenario: Add persisted across restart
- **WHEN** a monitor is added via the API and the binary is restarted
- **THEN** the new monitor appears in the monitor list after restart

#### Scenario: Delete persisted across restart
- **WHEN** a monitor is deleted via the API and the binary is restarted
- **THEN** the deleted monitor does NOT appear after restart

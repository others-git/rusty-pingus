## ADDED Requirements

### Requirement: Monitors have an enabled flag
Every monitor SHALL have an `enabled` boolean that defaults to `true` when omitted, so existing configuration files without the key load as enabled. A monitor with `enabled = false` SHALL be retained in configuration (and keep its stored history) but treated as paused. The flag SHALL round-trip through the config file (a disabled monitor stays disabled across reloads).

#### Scenario: Missing flag defaults to enabled
- **WHEN** a monitor entry omits `enabled`
- **THEN** the monitor loads as enabled

#### Scenario: Disabled monitor is retained, not removed
- **WHEN** a monitor has `enabled = false`
- **THEN** it remains in the configuration and its stored history is preserved

#### Scenario: Disabled state persists across reload
- **WHEN** a monitor is disabled and the configuration is reloaded
- **THEN** the monitor is still disabled

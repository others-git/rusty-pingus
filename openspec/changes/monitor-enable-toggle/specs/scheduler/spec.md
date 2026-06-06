## ADDED Requirements

### Requirement: Disabled monitors are not scheduled
The scheduler SHALL NOT run a probe task for a monitor whose `enabled` flag is false. Enabling or disabling a monitor SHALL take effect through the existing hot-reload path without a restart: disabling a monitor stops its running probe task, and enabling a monitor starts one.

#### Scenario: Disabled monitor is not probed
- **WHEN** the scheduler starts (or reloads) with a monitor that is disabled
- **THEN** no probe task runs for that monitor and no new probe results are recorded for it

#### Scenario: Disabling stops probing live
- **WHEN** a running, enabled monitor is disabled
- **THEN** its probe task is stopped without restarting the application, and no further probes run for it

#### Scenario: Enabling resumes probing live
- **WHEN** a disabled monitor is enabled
- **THEN** a probe task is started for it without restarting the application

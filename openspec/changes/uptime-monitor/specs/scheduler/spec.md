## ADDED Requirements

### Requirement: Per-monitor independent probe scheduling
The system SHALL schedule each monitor as an independent async task that runs its probe at the monitor's configured interval, regardless of other monitors' states or timings.

#### Scenario: Monitor probes on schedule
- **WHEN** a monitor with `interval_secs = 60` is registered
- **THEN** its probe executes approximately every 60 seconds from the time it was first registered

#### Scenario: Probe overrun does not skip next cycle
- **WHEN** a probe takes longer than expected (but less than the interval)
- **THEN** the next probe runs `interval_secs` after the previous probe started (not after it ended), maintaining a consistent cadence

### Requirement: Initial probe on startup
The system SHALL execute an initial probe for each monitor immediately (or within a few seconds) upon startup, rather than waiting for the first full interval to elapse.

#### Scenario: First probe runs at startup
- **WHEN** the binary starts and monitors are registered
- **THEN** each monitor's first probe executes within 5 seconds of startup

### Requirement: Graceful shutdown
The system SHALL respond to SIGTERM and SIGINT by stopping the scheduler and allowing in-flight probes to complete (up to a configurable drain timeout) before exiting.

#### Scenario: SIGTERM triggers clean shutdown
- **WHEN** the process receives SIGTERM
- **THEN** no new probes are started, in-flight probes are awaited up to the drain timeout (default 10 seconds), and the process exits with code 0

#### Scenario: Drain timeout exceeded
- **WHEN** in-flight probes do not complete within the drain timeout
- **THEN** the process exits with code 0 and logs a warning about abandoned probes

### Requirement: Scheduler error isolation
The system SHALL ensure that a panic or unrecoverable error in one monitor's probe task does not stop the scheduler or affect other monitors.

#### Scenario: Panicking probe task is restarted
- **WHEN** a monitor's probe task panics
- **THEN** the scheduler logs the error, restarts the probe task for that monitor, and other monitors continue unaffected

### Requirement: Monitor count scalability
The system SHALL support at least 200 concurrently scheduled monitors without degrading probe timing accuracy beyond ±5 seconds of the configured interval.

#### Scenario: 200 monitors scheduled concurrently
- **WHEN** 200 monitors are defined in the config file
- **THEN** all monitors execute their probes within ±5 seconds of their configured interval under normal system load

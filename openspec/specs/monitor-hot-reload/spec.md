## Purpose

Defines how the scheduler reacts to monitor store changes at runtime, starting and stopping probe tasks without a binary restart via a watch channel.

## Requirements

### Requirement: Scheduler reacts to monitor store changes
The system SHALL update running probe tasks when the monitor store changes without requiring a binary restart. New monitors SHALL start probing immediately; removed monitors SHALL stop probing.

#### Scenario: New monitor starts probing after API add
- **WHEN** a monitor is added via `POST /api/monitors`
- **THEN** within 2 seconds a probe task is running for the new monitor and its first probe executes

#### Scenario: Deleted monitor stops probing
- **WHEN** a monitor is deleted via `DELETE /api/monitors/:name`
- **THEN** the probe task for that monitor is cancelled and no further probes are executed for it

#### Scenario: Unchanged monitors continue unaffected
- **WHEN** a monitor is added or deleted
- **THEN** probe tasks for all other monitors continue running on their existing schedule without interruption or timing disruption

### Requirement: Hot-reload uses watch channel
The system SHALL use a `tokio::watch` channel to communicate monitor list changes from the store to the scheduler. The scheduler SHALL detect a change on the receiver and diff the old and new lists.

#### Scenario: Watch channel fires on store write
- **WHEN** a write operation (add or delete) completes on the monitor store
- **THEN** the watch channel sends the updated monitor list to all receivers

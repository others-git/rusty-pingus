# Live Status Updates Spec

## Purpose

Defines server-pushed, near-real-time monitor status delivery to the dashboard via Server-Sent Events, with periodic polling retained as a graceful fallback and reconciliation mechanism. Monitor detail pages are explicitly excluded from auto-updating.

## Requirements

### Requirement: Status broadcast on probe completion
When a probe result is recorded, the system SHALL broadcast a lightweight status update (monitor name, status, response time, last-check time, failure reason, and detail) to any connected real-time subscribers, without performing additional database work for the broadcast.

#### Scenario: Update emitted after a probe
- **WHEN** a monitor's probe completes and its result is stored
- **THEN** a status update for that monitor is broadcast to subscribers

#### Scenario: No subscribers is harmless
- **WHEN** a probe completes and no client is subscribed
- **THEN** the broadcast is a no-op and probing continues normally

### Requirement: Real-time status stream endpoint
The system SHALL expose `GET /api/monitors/stream` as a Server-Sent Events endpoint that relays status updates to the client as they occur, keeping the connection alive through idle periods.

#### Scenario: Client receives live updates
- **WHEN** a client connects to the stream and a monitor's status subsequently updates
- **THEN** the client receives an event carrying that monitor's updated status

#### Scenario: Connection persists while idle
- **WHEN** no updates occur for a while
- **THEN** the stream connection is kept alive (not closed for inactivity)

#### Scenario: Disconnect is clean
- **WHEN** a client disconnects from the stream
- **THEN** the server stops sending to it without affecting probing or other clients

### Requirement: Dashboard updates in real time with polling fallback
The dashboard SHALL subscribe to the status stream and update each monitor's card in place as updates arrive (status, response time, last-checked time, up/down counts). The periodic poll SHALL be retained as a fallback and to reconcile the monitor list and longer-window metrics, so the dashboard remains correct if the stream is unavailable or interrupted.

#### Scenario: Card updates without reload or waiting for the poll
- **WHEN** a monitor changes status and the dashboard is open with a live connection
- **THEN** that monitor's card reflects the new status within about a second, without a page reload and without waiting for the next poll

#### Scenario: Falls back to polling
- **WHEN** the live stream is unavailable or drops
- **THEN** the dashboard still updates via its periodic poll

#### Scenario: List membership reconciled by the poll
- **WHEN** a monitor is added or removed
- **THEN** the dashboard's set of cards is reconciled by the periodic poll (live updates apply to existing cards)

### Requirement: Detail pages do not auto-update
Monitor detail pages SHALL NOT subscribe to the stream or auto-refresh; they load on navigation and update only on explicit user action.

#### Scenario: Detail page stays static
- **WHEN** a user is on a monitor detail page and the monitor's status changes
- **THEN** the detail page does not change until the user reloads or navigates

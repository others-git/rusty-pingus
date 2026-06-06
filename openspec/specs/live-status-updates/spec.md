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

### Requirement: Detail pages update live with polling fallback
The monitor detail page SHALL subscribe to the status stream (`GET /api/monitors/stream`) and update in place as status updates arrive for the monitor it is displaying, without a page reload. A periodic poll SHALL be retained as a fallback so the detail page stays current when the stream is unavailable or interrupted. Updates for other monitors SHALL be ignored by the page.

#### Scenario: Detail page reflects a new probe without reload
- **WHEN** a user is on a monitor's detail page with a live connection and that monitor records a new probe result
- **THEN** the page's status, detail, and metrics update in place within about a second, without a reload

#### Scenario: Newly available data fills in
- **WHEN** a detail page is opened for a monitor that has no data in the active window yet, and probe results subsequently arrive
- **THEN** the page populates its metrics and chart from the arriving data without the user reloading

#### Scenario: Updates for other monitors are ignored
- **WHEN** the stream delivers an update for a monitor other than the one being viewed
- **THEN** the detail page does not change

#### Scenario: Falls back to polling
- **WHEN** the live stream is unavailable or drops
- **THEN** the detail page still stays current via its periodic poll

### Requirement: State-timeline detail pages do not auto-reload the chart per probe
On a state-timeline monitor detail page (public-IP, border), the page SHALL update the header's current value (e.g. current public IP, fault class) live as probes arrive, but SHALL NOT reload or re-render the chart/timeline on every probe. The chart SHALL reload only on user action (selecting a window, zoom/pan, or an explicit refresh) or when the tracked value actually changes. This prevents continuous re-rendering ("flashing") for monitors whose value rarely changes.

#### Scenario: Header updates live without the chart flashing
- **WHEN** a public-IP monitor's detail page is open and successive probes return the same IP
- **THEN** the header reflects the latest check, and the IP timeline is not re-rendered for each probe (no flashing)

#### Scenario: Chart updates when the tracked value changes
- **WHEN** a public-IP monitor's observed IP changes while its detail page is open
- **THEN** the timeline updates to reflect the new value

#### Scenario: Response-chart monitors still update live
- **WHEN** a response-time monitor (e.g. ICMP/HTTP/TCP) detail page is open
- **THEN** its live-update behavior is unchanged by this requirement (it applies only to state-timeline monitors)

## ADDED Requirements

### Requirement: Detail charts use server-anchored time windows
The monitor detail page SHALL anchor its time windows (the active 1h/24h/7d/30d window and any derived ranges sent to the history and series endpoints) to the most recent **server-recorded** probe timestamp, not to the client's clock. When no probe data exists, it MAY fall back to the client clock. This keeps the chart, state-timeline, and metrics correct when the client and server clocks differ.

#### Scenario: Chart renders despite client/server clock skew
- **WHEN** the browser's clock differs from the server's clock and the detail page loads a window
- **THEN** the requested range is anchored to the latest server-recorded probe time, so the window contains the monitor's data and the chart/timeline renders it (rather than appearing empty)

#### Scenario: Window selection stays anchored to data time
- **WHEN** a user selects an uptime window (e.g. 24h) on the detail page
- **THEN** the loaded range ends at the latest server-recorded probe time and spans the selected duration back from there

### Requirement: Public-IP detail view is purpose-built
The public-IP monitor detail view SHALL be oriented around tracking the external IP — showing the current IP, how long it has been stable, IP-change count, uptime, and the IP-over-time timeline — and SHALL NOT present response-time/latency metrics or labels, which are not meaningful for this monitor type.

#### Scenario: No response-time framing for public-IP
- **WHEN** a user views a public-IP monitor's detail page
- **THEN** the metrics strip and chart show IP-oriented information (current IP, stability, IP changes, uptime) and no response-time value, latency axis, or "response time" labeling

#### Scenario: IP timeline is the primary chart
- **WHEN** a public-IP monitor's detail page renders its primary chart
- **THEN** it shows the IP-over-time state timeline, not a response-time line

## MODIFIED Requirements

### Requirement: Dashboard overview page
The system SHALL serve a dashboard page at `/` showing the current status of all configured monitors including name, protocol, endpoint, current status (up/down), last check time, and uptime percentage for the last 24 hours. The page SHALL include a sticky header displaying a global summary (total monitors, count up, count down), and monitor cards SHALL be rendered using Alpine.js reactive state populated from the `/api/monitors` JSON endpoint. Each card SHALL display a Font Awesome protocol icon, an animated pulse indicator for `up` status, a styled status badge, the response time in ms, and the 24h uptime percentage.

#### Scenario: All monitors shown
- **WHEN** a user opens the dashboard
- **THEN** every configured monitor appears as a dark card with its current status indicated visually (emerald pulse for up, red badge for down, amber for pending)

#### Scenario: Last check time displayed
- **WHEN** a monitor has completed at least one probe
- **THEN** the dashboard shows the time of the most recent probe result for that monitor, formatted as a relative time (e.g., "2 minutes ago")

#### Scenario: No probes yet
- **WHEN** a monitor has not yet completed any probes
- **THEN** the dashboard shows that monitor with an amber "pending" badge and no response time

#### Scenario: Global summary header shows counts
- **WHEN** the dashboard loads
- **THEN** a sticky header bar displays "X / Y Monitors Up" and the total count of monitors in "down" state

### Requirement: Monitor detail page
The system SHALL serve a detail page at `/monitors/:name` showing the full probe history, a dark-themed gradient response time Chart.js chart, and uptime percentages for multiple windows (1h, 24h, 7d, 30d) rendered as SVG gauge rings. Alpine.js SHALL manage data fetching and state for this page.

#### Scenario: Response time chart displayed
- **WHEN** a user opens a monitor detail page
- **THEN** a Chart.js line chart with a dark background (`slate-900`), a cyan/teal gradient fill, and no visible gridlines renders the last 24 hours of response times

#### Scenario: Uptime windows shown as gauge rings
- **WHEN** a user views a monitor detail page
- **THEN** four SVG circular gauge rings display uptime percentages for 1h, 24h, 7d, and 30d windows, each colored by threshold (emerald/amber/red)

#### Scenario: History table with styled rows
- **WHEN** a user views the probe history table on the detail page
- **THEN** each row shows a colored status badge, timestamp, response time, and failure reason (if any), with alternating `slate-800`/`slate-900` row backgrounds

### Requirement: Dashboard auto-refresh
The system SHALL poll the `/api/monitors` endpoint every 30 seconds using Alpine.js reactive state and update the dashboard display without a full page reload. A visual "last updated" timestamp SHALL be shown and updated on each successful poll.

#### Scenario: Status updates without reload
- **WHEN** a monitor changes status between polls
- **THEN** the Alpine.js reactive state update causes the card to re-render with the new status at the next polling interval without a browser refresh

#### Scenario: Last updated timestamp refreshes
- **WHEN** a successful poll completes
- **THEN** the "Last updated" display in the header updates to reflect the current time

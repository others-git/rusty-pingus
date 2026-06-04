## Purpose

Defines the embedded web server, dashboard UI, and JSON API endpoints for viewing monitor status, probe history, and uptime statistics.

## Requirements

### Requirement: Embedded web server
The system SHALL start an HTTP server on a configurable bind address and port, serving the web dashboard and JSON API from within the binary.

#### Scenario: Server starts on configured address
- **WHEN** the binary starts with `web.bind = "0.0.0.0:3000"` in config
- **THEN** the web server is accessible at `http://<host>:3000`

#### Scenario: Server bind failure
- **WHEN** the configured port is already in use
- **THEN** the binary exits with a non-zero status and a clear error message indicating the bind failure

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

### Requirement: JSON API for monitor status
The system SHALL expose a `GET /api/monitors` endpoint returning the current status of all monitors as a JSON array.

#### Scenario: API returns all monitors
- **WHEN** `GET /api/monitors` is requested
- **THEN** the response is a JSON array where each element contains `name`, `protocol`, `endpoint`, `status`, `last_checked_at`, `response_time_ms`, and `uptime_24h`

### Requirement: JSON API for monitor history
The system SHALL expose a `GET /api/monitors/:name/history` endpoint returning paginated probe results for the specified monitor with optional `from`, `to`, and `limit` query parameters.

#### Scenario: History with default limit
- **WHEN** `GET /api/monitors/my-api/history` is requested without parameters
- **THEN** the 100 most recent probe results are returned as a JSON array

#### Scenario: History with time range
- **WHEN** `GET /api/monitors/my-api/history?from=<iso8601>&to=<iso8601>` is requested
- **THEN** only results within that time range are returned

#### Scenario: Unknown monitor name
- **WHEN** `GET /api/monitors/nonexistent/history` is requested
- **THEN** the API returns HTTP 404 with a JSON error body

### Requirement: Dashboard auto-refresh
The system SHALL poll the `/api/monitors` endpoint every 30 seconds using Alpine.js reactive state and update the dashboard display without a full page reload. A visual "last updated" timestamp SHALL be shown and updated on each successful poll.

#### Scenario: Status updates without reload
- **WHEN** a monitor changes status between polls
- **THEN** the Alpine.js reactive state update causes the card to re-render with the new status at the next polling interval without a browser refresh

#### Scenario: Last updated timestamp refreshes
- **WHEN** a successful poll completes
- **THEN** the "Last updated" display in the header updates to reflect the current time

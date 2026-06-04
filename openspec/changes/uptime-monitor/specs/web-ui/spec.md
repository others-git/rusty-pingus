## ADDED Requirements

### Requirement: Embedded web server
The system SHALL start an HTTP server on a configurable bind address and port, serving the web dashboard and JSON API from within the binary.

#### Scenario: Server starts on configured address
- **WHEN** the binary starts with `web.bind = "0.0.0.0:3000"` in config
- **THEN** the web server is accessible at `http://<host>:3000`

#### Scenario: Server bind failure
- **WHEN** the configured port is already in use
- **THEN** the binary exits with a non-zero status and a clear error message indicating the bind failure

### Requirement: Dashboard overview page
The system SHALL serve a dashboard page at `/` showing the current status of all configured monitors including name, protocol, endpoint, current status (up/down), last check time, and uptime percentage for the last 24 hours.

#### Scenario: All monitors shown
- **WHEN** a user opens the dashboard
- **THEN** every configured monitor appears as a card or row with its current status indicated visually (green/red)

#### Scenario: Last check time displayed
- **WHEN** a monitor has completed at least one probe
- **THEN** the dashboard shows the time of the most recent probe result for that monitor

#### Scenario: No probes yet
- **WHEN** a monitor has not yet completed any probes
- **THEN** the dashboard shows that monitor with status `pending`

### Requirement: Monitor detail page
The system SHALL serve a detail page at `/monitors/:name` showing the full probe history, a response time chart, and uptime percentages for multiple windows (1h, 24h, 7d, 30d).

#### Scenario: Response time chart displayed
- **WHEN** a user opens a monitor detail page
- **THEN** a time-series chart of response times over the last 24 hours is rendered

#### Scenario: Uptime windows shown
- **WHEN** a user views a monitor detail page
- **THEN** uptime percentages for 1h, 24h, 7d, and 30d windows are displayed

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
The system SHALL poll the `/api/monitors` endpoint every 30 seconds and update the dashboard display without a full page reload.

#### Scenario: Status updates without reload
- **WHEN** a monitor changes status between polls
- **THEN** the dashboard card updates its display at the next polling interval without requiring a manual browser refresh

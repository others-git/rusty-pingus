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
The system SHALL serve a dashboard page at `/` showing the current status of all configured monitors including name, protocol, endpoint, current status (up/down), last check time, and uptime percentage for the last 24 hours. The page SHALL include a sticky header displaying a global summary (total monitors, count up, count down), and monitor cards SHALL be rendered using Alpine.js reactive state populated from the `/api/monitors` JSON endpoint. Each card SHALL display a Font Awesome protocol icon, an animated pulse indicator for `up` status, a styled status badge, the response time in ms, and the 24h uptime percentage. Each monitor card SHALL include a delete button that, on confirmation, calls `DELETE /api/monitors/:name` and removes the card from the list. A deleted monitor SHALL NOT reappear on subsequent polls. The header SHALL include an "Add Monitor" button that opens the add-monitor modal.

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

#### Scenario: Delete monitor with confirmation
- **WHEN** a user clicks the delete button on a monitor card
- **THEN** a confirmation dialog appears; on confirm, `DELETE /api/monitors/:name` is called and the card is removed from the dashboard without a page reload

#### Scenario: Deleted monitor does not reappear
- **WHEN** a monitor is deleted and the dashboard performs its next auto-refresh poll
- **THEN** the deleted monitor remains absent from the dashboard (its probe history has been purged), and it is not shown again

#### Scenario: Add Monitor button opens modal
- **WHEN** a user clicks the "Add Monitor" button in the header
- **THEN** the add-monitor modal opens

### Requirement: Add monitor modal form
The system SHALL provide a modal form for adding new monitors, accessible from the dashboard header. The form SHALL support HTTP, TCP, and ICMP protocols via tabs or a protocol selector. Interval and timeout inputs SHALL be expressed in milliseconds and submitted as `interval_ms` and `timeout_ms`. The form SHALL validate inputs inline and display errors next to the relevant field. On success, the modal SHALL close and the new monitor SHALL appear in the dashboard.

#### Scenario: HTTP monitor added successfully
- **WHEN** a user fills in the HTTP form (name, URL, interval in ms) and submits
- **THEN** the monitor appears in the dashboard and the modal closes

#### Scenario: Validation error shown inline
- **WHEN** a user submits the form with an invalid URL
- **THEN** the URL field shows an error message and the form does not submit

#### Scenario: Interval below minimum shown inline
- **WHEN** a user submits the form with an interval below 5000 ms
- **THEN** the interval field shows an error message and the form does not submit

#### Scenario: Duplicate name rejected
- **WHEN** a user submits a name that already exists
- **THEN** the name field shows "A monitor with this name already exists" and the form does not submit

#### Scenario: Protocol selector changes visible fields
- **WHEN** a user selects "TCP" in the protocol selector
- **THEN** host and port fields are shown and the URL field is hidden

#### Scenario: Modal dismissible
- **WHEN** a user clicks outside the modal or presses Escape
- **THEN** the modal closes without saving

### Requirement: Monitor detail page
The system SHALL serve a data-first detail page at `/monitors/:name`. The page SHALL lead with the monitor's live status and a compact metrics strip (latest response time, 24h uptime, last check time as relative time, and sample/total-check count). The primary element SHALL be a response-time chart rendered on a time-scale x-axis that supports zooming (mouse wheel and drag) and panning, with a control to reset the zoom to the active window. The chart data SHALL come from the aggregated series endpoint (not raw rows). Uptime percentages for 1h, 24h, 7d, and 30d SHALL be shown as compact, clickable selectors; clicking one SHALL set the chart's active time range to that window. A raw probe history table SHALL remain available as a secondary, capped element. Alpine.js SHALL manage data fetching and state for this page.

#### Scenario: Metrics strip shown first
- **WHEN** a user opens a monitor detail page
- **THEN** the live status and a compact metrics strip (latest response time, 24h uptime, last check as relative time, sample count) appear above the chart

#### Scenario: Response time chart on a zoomable time scale
- **WHEN** a user opens a monitor detail page
- **THEN** a dark-themed Chart.js line chart renders response time over a true time-scale x-axis, and the user can zoom with the mouse wheel or by dragging and can pan the visible range

#### Scenario: Reset zoom
- **WHEN** a user has zoomed or panned the chart and activates the reset control
- **THEN** the chart returns to the currently active window

#### Scenario: Clickable uptime window sets the chart range
- **WHEN** a user clicks the 7d uptime window
- **THEN** the chart's active range is set to the last 7 days, the corresponding aggregated data is loaded, and the 7d window is visually highlighted as selected

#### Scenario: Uptime windows shown as selectors
- **WHEN** a user views a monitor detail page
- **THEN** uptime percentages for 1h, 24h, 7d, and 30d are displayed as compact controls colored by threshold (emerald/amber/red), the active window indicated

#### Scenario: History table is secondary
- **WHEN** a user scrolls past the chart
- **THEN** a capped raw probe history table shows recent rows with a colored status badge, timestamp, response time, and failure reason

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

### Requirement: JSON API for aggregated response-time series
The system SHALL expose `GET /api/monitors/:name/series` returning response-time and availability data bucketed over a `[from, to]` time range into a bounded number of points, suitable for charting any window without returning raw rows. Query parameters `from`, `to`, and `buckets` SHALL be optional with sensible defaults.

#### Scenario: Series returns bucketed points
- **WHEN** `GET /api/monitors/my-api/series?from=<iso8601>&to=<iso8601>&buckets=300` is requested
- **THEN** a JSON array of at most ~300 buckets is returned, each with a bucket start timestamp, average/min/max response time, sample count, and up-ratio

#### Scenario: Default range and bounded buckets
- **WHEN** `GET /api/monitors/my-api/series` is requested without parameters
- **THEN** the last 24 hours are returned as a bounded number of buckets with HTTP 200

#### Scenario: Bucket count is clamped
- **WHEN** a series request specifies an excessively large `buckets` value
- **THEN** the server clamps it to a safe maximum before querying

#### Scenario: Monitor with no results in range
- **WHEN** `GET /api/monitors/my-api/series` is requested for a range containing no probe results
- **THEN** an empty JSON array is returned with HTTP 200

### Requirement: Dynamic chart resolution
The monitor detail response-time chart SHALL load data at a resolution matched to the visible time range. When the user zooms or pans, the chart SHALL refetch data for the currently visible range so that range is rendered at approximately the chart's point budget rather than the originally loaded window's resolution. When the visible range contains few enough probes to render individually, the chart SHALL display raw probe points; otherwise it SHALL display aggregated buckets. Refetches SHALL be debounced, and a loading indicator SHALL be shown while finer data loads. Resetting the zoom SHALL restore the active window and its resolution.

#### Scenario: Zooming in loads finer data
- **WHEN** a user zooms into a sub-range of the chart
- **THEN** after the gesture settles, data for the visible range is refetched and the chart renders that range at higher temporal resolution than before the zoom

#### Scenario: Raw probes appear at deep zoom
- **WHEN** the user zooms in until the visible range contains at most the raw-point budget of probes
- **THEN** the chart renders each individual probe as a point (raw mode) rather than aggregated buckets

#### Scenario: Aggregated buckets when zoomed out
- **WHEN** the visible range contains more probes than can be rendered individually
- **THEN** the chart renders aggregated buckets (avg/min/max, up-ratio) for the visible range

#### Scenario: Panning loads the newly visible range
- **WHEN** a user pans the chart to a different time range
- **THEN** after the gesture settles, data for the newly visible range is fetched and rendered

#### Scenario: Refetches are debounced
- **WHEN** a user performs rapid successive zoom or pan gestures
- **THEN** intermediate refetches are coalesced so the server is not queried on every interaction, and only the final visible range is loaded

#### Scenario: Reset restores the active window
- **WHEN** a user activates the reset control after zooming or panning
- **THEN** the chart returns to the active uptime window and reloads that window's resolution

### Requirement: Dashboard auto-refresh
The system SHALL poll the `/api/monitors` endpoint every 30 seconds using Alpine.js reactive state and update the dashboard display without a full page reload. A visual "last updated" timestamp SHALL be shown and updated on each successful poll.

#### Scenario: Status updates without reload
- **WHEN** a monitor changes status between polls
- **THEN** the Alpine.js reactive state update causes the card to re-render with the new status at the next polling interval without a browser refresh

#### Scenario: Last updated timestamp refreshes
- **WHEN** a successful poll completes
- **THEN** the "Last updated" display in the header updates to reflect the current time

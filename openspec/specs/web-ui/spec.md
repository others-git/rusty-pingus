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
The monitor detail response-time chart SHALL load data at a resolution matched to the visible time range. On load (and when an uptime window is selected) the chart SHALL fit its x-axis to the extent of the loaded data so the points fill the plot area rather than being compressed against an edge. When the user zooms or pans, the chart SHALL refetch data for the currently visible range so that range is rendered at approximately the chart's point budget rather than the originally loaded window's resolution. Panning SHALL move the view freely through time (it SHALL NOT be constrained to the initial window), and a zoom/pan-triggered refetch SHALL NOT reset the view position. When the visible range contains few enough probes to render individually, the chart SHALL display raw probe points; otherwise it SHALL display aggregated buckets. Refetches SHALL be debounced, and a loading indicator SHALL be shown while finer data loads. Resetting the zoom SHALL restore the active window and its resolution.

#### Scenario: Data fills the chart on load
- **WHEN** the monitor detail page loads (or a user selects an uptime window)
- **THEN** the loaded data fills the plot area, fitted to the data's time extent, rather than being bunched against the left or right edge

#### Scenario: Panning moves the view
- **WHEN** a user drags to pan the chart
- **THEN** the visible time range moves accordingly, and after the gesture settles data for the newly visible range is fetched and rendered

#### Scenario: Gesture position preserved across refetch
- **WHEN** a zoom or pan gesture triggers a refetch that changes the rendered resolution
- **THEN** the view stays at the position the user left it (the refetch swaps data without snapping the view back)

#### Scenario: Zooming in loads finer data
- **WHEN** a user zooms into a sub-range of the chart
- **THEN** after the gesture settles, data for the visible range is refetched and the chart renders that range at higher temporal resolution than before the zoom

#### Scenario: Raw probes appear at deep zoom
- **WHEN** the user zooms in until the visible range contains at most the raw-point budget of probes
- **THEN** the chart renders each individual probe as a point (raw mode) rather than aggregated buckets

#### Scenario: Aggregated buckets when zoomed out
- **WHEN** the visible range contains more probes than can be rendered individually
- **THEN** the chart renders aggregated buckets (avg/min/max, up-ratio) for the visible range

#### Scenario: Refetches are debounced
- **WHEN** a user performs rapid successive zoom or pan gestures
- **THEN** intermediate refetches are coalesced so the server is not queried on every interaction, and only the final visible range is loaded

#### Scenario: Reset restores the active window
- **WHEN** a user activates the reset control after zooming or panning
- **THEN** the chart returns to the active uptime window, re-fitted to that window's data

### Requirement: Downtime visualization on the response chart
The monitor detail response-time chart SHALL make downtime unmistakable: at points/intervals where the monitor is down (a raw probe with status `down`, or an aggregated interval with an up-ratio below 1), the response-time line SHALL be interrupted (a gap rather than a connecting segment), and the down span SHALL be shaded with a red vertical band over its time range.

#### Scenario: Outage shown as a gap plus red band
- **WHEN** the visible data contains a downtime span
- **THEN** the line does not connect across the outage (it shows a gap), and a red vertical band is drawn spanning the outage's time range

#### Scenario: No downtime
- **WHEN** the visible data contains no down samples
- **THEN** the line is continuous and no red bands are drawn

#### Scenario: Downtime band on aggregated view
- **WHEN** the chart is showing aggregated buckets and a bucket has loss (up-ratio below 1)
- **THEN** that bucket's time span is shaded with a red band

### Requirement: Downtime markers stay visible and are labeled
Downtime on the monitor chart SHALL be marked in a way that remains visible at any zoom level (a fixed-size marker, not only a duration-proportional band). Brief drops and sustained outages SHALL be visually distinct: a brief drop is shown as a thin vertical marker, while a sustained outage (lasting beyond a short threshold) is shown as a filled band labeled with its start time and duration. Hovering a downtime marker SHALL reveal its time — a single timestamp for a brief drop, and the start→end range with duration for a sustained outage.

#### Scenario: Brief drop stays visible when zoomed in
- **WHEN** the user zooms in on a single dropped probe
- **THEN** a thin vertical downtime marker remains visible at that time (it does not shrink to sub-pixel and disappear)

#### Scenario: Sustained outage is fat and labeled
- **WHEN** an outage lasts beyond the sustained threshold
- **THEN** it is rendered as a filled red band labeled with its start time and duration

#### Scenario: Hover reveals the outage time
- **WHEN** the user hovers a downtime marker
- **THEN** its timestamp is shown (a single time for a brief drop, or the start→end range and duration for a sustained outage)

#### Scenario: Outage locatable when zoomed far out
- **WHEN** the chart is zoomed out far enough that an outage's band would be sub-pixel
- **THEN** a fixed-size marker still indicates the outage's location on the timeline

### Requirement: Response chart pan and zoom
The monitor detail response-time chart SHALL provide built-in pan and zoom: the user SHALL be able to zoom with the mouse wheel, pan by dragging within the plot, and adjust the visible range via a draggable range slider. Zooming and panning SHALL drive the dynamic-resolution refetch so the visible range is rendered at an appropriate resolution.

#### Scenario: Wheel zoom
- **WHEN** the user scrolls the wheel over the chart
- **THEN** the chart zooms the time axis about the cursor, and after the gesture settles the visible range is refetched at the matching resolution

#### Scenario: Drag to pan
- **WHEN** the user drags within the plot area
- **THEN** the visible time range moves with the drag, and the newly visible range is fetched and rendered

#### Scenario: Range slider
- **WHEN** the user drags the range slider beneath the chart
- **THEN** the visible time range updates accordingly and the corresponding data is loaded

### Requirement: Chart tooltip is robust at any hover position
The monitor detail chart tooltip SHALL render without error at any hover position, including over outage gaps where there is no data point. It SHALL show the hovered time and the response time, or "no response" when there is no value at that point.

#### Scenario: Hover over a data point
- **WHEN** the user hovers over a point on the response-time line
- **THEN** the tooltip shows that point's time and response time, with no console error

#### Scenario: Hover over an outage gap
- **WHEN** the user hovers over a time where the line is interrupted (an outage)
- **THEN** the tooltip renders showing the time and "no response", without throwing

### Requirement: Dashboard auto-refresh
The system SHALL poll the `/api/monitors` endpoint every 30 seconds using Alpine.js reactive state and update the dashboard display without a full page reload. A visual "last updated" timestamp SHALL be shown and updated on each successful poll.

#### Scenario: Status updates without reload
- **WHEN** a monitor changes status between polls
- **THEN** the Alpine.js reactive state update causes the card to re-render with the new status at the next polling interval without a browser refresh

#### Scenario: Last updated timestamp refreshes
- **WHEN** a successful poll completes
- **THEN** the "Last updated" display in the header updates to reflect the current time

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

### Requirement: Add public-IP and border monitors from the UI
The add-monitor modal SHALL offer the public-IP and border monitor types alongside HTTP/TCP/ICMP, showing the fields relevant to each (public-IP: an optional service URL; border: an optional gateway and an upstream target), and submit them to `POST /api/monitors`.

#### Scenario: Public-IP type selectable
- **WHEN** a user opens the add-monitor modal and selects the public-IP type
- **THEN** the form shows the public-IP fields and can create a public-IP monitor

#### Scenario: Border type selectable
- **WHEN** a user selects the border type
- **THEN** the form shows gateway and upstream fields and can create a border monitor

### Requirement: Display per-probe detail
The dashboard and monitor detail views SHALL surface a probe's detail when present — e.g. the current public IP for a public-IP monitor, or the fault localization for a border monitor — without disrupting the existing card/detail layout.

#### Scenario: Public IP shown
- **WHEN** a public-IP monitor has a recorded IP
- **THEN** that IP is shown on its card / detail view

#### Scenario: Border status shown
- **WHEN** a border monitor has a fault classification (e.g. ISP down vs LAN down)
- **THEN** that classification is shown on its card / detail view

#### Scenario: No detail, no clutter
- **WHEN** a monitor has no detail (e.g. a plain HTTP check)
- **THEN** no extra detail element is shown for it

### Requirement: Detail page refreshes live without disrupting interaction
The monitor detail page SHALL refresh in place as new probe results arrive for the monitor being viewed (via the live status stream, with a periodic poll as fallback). The refresh SHALL update the header status/detail and the metrics strip, and SHALL update the active-window chart or state-timeline data so newly arrived samples appear without a manual reload. A live refresh SHALL NOT reset or interfere with the user's current zoom/pan position, and SHALL NOT interrupt a pending zoom/pan-triggered refetch.

#### Scenario: Active window fills in as data arrives
- **WHEN** the detail page is showing a window that currently has no samples and new probe results arrive for the monitor
- **THEN** the metrics strip and chart/timeline update in place to show the arriving data, without the user reloading

#### Scenario: Live refresh preserves zoom and pan
- **WHEN** the user has zoomed or panned the chart and a live update arrives
- **THEN** the view stays at the user's current position rather than snapping back to the full window

#### Scenario: Header status reflects the latest probe
- **WHEN** a monitor's status or detail changes while its detail page is open with a live connection
- **THEN** the page's status indicator and type-specific detail (e.g. current public IP) update within about a second

### Requirement: Detail view distinguishes no-data from unreachable
The monitor detail view SHALL visually distinguish a window that has no samples yet ("no data" / "collecting") from a monitor whose most recent probe failed ("down" / "unreachable"). When a window contains no samples, the public-IP strip and the chart/timeline area SHALL present a neutral no-data state rather than implying the monitor or its target is unreachable.

#### Scenario: Empty window shows a no-data state
- **WHEN** a public-IP monitor's detail page is opened for a window that has no recorded probes yet
- **THEN** the "Current IP" field shows a neutral no-data/collecting state (not "unreachable"), and the chart area indicates there is no data in the window

#### Scenario: Failed latest probe shows unreachable
- **WHEN** a public-IP monitor has samples in the window and its most recent probe failed (no IP recorded)
- **THEN** the "Current IP" field shows "unreachable", distinct from the no-data state

### Requirement: Traceroute detail page with per-hop table
The monitor detail page SHALL render a purpose-built layout for `traceroute` monitors: a table with one row per hop showing the hop number, the hop address (with reachable / unreachable state), and the hop's average, minimum, and maximum round-trip time over the active time range. This layout SHALL be used instead of the response-time line chart and state timeline used by other monitor types.

#### Scenario: Hops shown as table rows
- **WHEN** a user opens a traceroute monitor's detail page
- **THEN** each hop on the path is shown as a table row with its number, address, reachable state, and avg/min/max RTT

#### Scenario: Unreachable hop is visibly distinguished
- **WHEN** a hop did not respond over the active range
- **THEN** its row indicates the unreachable state rather than showing a misleading latency

### Requirement: Per-hop latency graph
The traceroute detail page SHALL visualize per-hop latency as a single connected graph aligned to the hop rows: for each hop a band spanning its minimum-to-maximum RTT and a marker at its average, with the per-hop averages joined by a line down the hops, so relative latency across the path is readable at a glance. A non-responding hop SHALL be shown without a band or marker rather than as zero latency.

#### Scenario: Latency shown as a connected per-hop graph
- **WHEN** the hop table renders
- **THEN** each hop shows a min–max band and an average marker, and the per-hop averages are connected into one graph down the rows

#### Scenario: Non-responding hop leaves a gap
- **WHEN** a hop did not respond over the active range
- **THEN** it shows no band or average marker rather than a latency of zero

### Requirement: Resizable time-range brush for traceroute data
The traceroute detail page SHALL provide a resizable scroll/brush control that selects the time range used to compute the per-hop table, operating within the monitor's retained data. Adjusting the control SHALL re-aggregate the table over the selected range. The page SHALL NOT use the fixed 1h/24h/7d/30d uptime windows for this monitor type.

#### Scenario: Adjusting the brush re-aggregates the table
- **WHEN** the user resizes or moves the time-range brush
- **THEN** the per-hop table's reachability and min/avg/max recompute over the newly selected range

#### Scenario: No fixed uptime windows for traceroute
- **WHEN** a traceroute monitor's detail page renders
- **THEN** it presents the retention-bounded brush control rather than the 1h/24h/7d/30d window selectors

### Requirement: Add-monitor form supports traceroute
The add-monitor form SHALL allow creating a `traceroute` monitor, collecting the target host, check interval (with the 500 ms minimum enforced), per-hop timeout, maximum hops, queries-per-hop, and retention.

#### Scenario: Create a traceroute monitor from the UI
- **WHEN** a user selects the traceroute type and submits a target with valid fields
- **THEN** a traceroute monitor is created with those settings, with the interval clamped to at least 500 ms

### Requirement: Dashboard enable/disable toggle with paused state
Each monitor on the dashboard SHALL provide a control to enable or disable it, calling the toggle endpoint and reflecting the new state without a full reload. A disabled monitor SHALL be presented in a distinct, de-emphasized "paused" state — visually separate from the "down" state — so a paused monitor is never mistaken for a failing one. The disabled state SHALL be driven by the monitor's `enabled` flag (from the list), not by live probe events (which do not occur while paused).

#### Scenario: Toggle a monitor from the dashboard
- **WHEN** a user activates a monitor's enable/disable control
- **THEN** the monitor's enabled state changes via the toggle endpoint and its card updates to reflect the new state

#### Scenario: Paused monitor is visually distinct from down
- **WHEN** a monitor is disabled
- **THEN** its card shows a de-emphasized "paused" state distinct from the "down" state, and it is not shown as failing

#### Scenario: Paused state shown without waiting for a probe
- **WHEN** a disabled monitor's card renders (no live probe events arrive while paused)
- **THEN** the paused state is shown based on the monitor's enabled flag from the list/poll

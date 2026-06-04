## MODIFIED Requirements

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

## ADDED Requirements

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

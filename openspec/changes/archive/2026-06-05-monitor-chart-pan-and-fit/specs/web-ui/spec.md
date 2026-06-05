## MODIFIED Requirements

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

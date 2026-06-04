## ADDED Requirements

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

## ADDED Requirements

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

## ADDED Requirements

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

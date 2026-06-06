## ADDED Requirements

### Requirement: Traceroute detail page with per-hop table
The monitor detail page SHALL render a purpose-built layout for `traceroute` monitors: a table with one row per hop showing the hop number, the hop address (with reachable / unreachable state), and the hop's last, average, minimum, and maximum round-trip time over the active time range. This layout SHALL be used instead of the response-time line chart and state timeline used by other monitor types.

#### Scenario: Hops shown as table rows
- **WHEN** a user opens a traceroute monitor's detail page
- **THEN** each hop on the path is shown as a table row with its number, address, reachable state, and last/avg/min/max RTT

#### Scenario: Unreachable hop is visibly distinguished
- **WHEN** a hop did not respond over the active range
- **THEN** its row indicates the unreachable state rather than showing a misleading latency

### Requirement: Relative-latency waterfall bar per hop
Each hop row SHALL include a bar whose horizontal length is proportional to that hop's latency, scaled to the slowest hop in view, so that slower hops extend farther to the right and the rows form a left-to-right latency waterfall.

#### Scenario: Bar length reflects hop latency
- **WHEN** the hop table renders
- **THEN** a hop with higher latency draws a longer bar than a hop with lower latency, scaled to the slowest hop shown

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

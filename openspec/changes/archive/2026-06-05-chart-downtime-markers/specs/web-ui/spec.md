## ADDED Requirements

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

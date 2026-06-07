## ADDED Requirements

### Requirement: Detail page uses a retention-bounded time brush
The monitor detail page SHALL provide a resizable time-range brush spanning the monitor's retained data (`[now − retention, now]`) as the control for the chart's active range, replacing the fixed 1h/24h/7d/30d range buttons. Adjusting the brush SHALL reload the chart/timeline for the selected range. Uptime percentages MAY still be shown as read-only statistics. This applies to response-time and state-timeline monitor detail pages.

#### Scenario: Brush sets the chart range
- **WHEN** a user resizes or moves the detail-page time brush
- **THEN** the chart/timeline reloads for the newly selected range

#### Scenario: Brush is bounded by retention
- **WHEN** the detail page renders for a monitor with a given retention
- **THEN** the brush spans at most the retained window and cannot select a range with no retained data

#### Scenario: Uptime figures remain as stats
- **WHEN** a user views the detail page
- **THEN** the 1h/24h/7d/30d uptime percentages are still shown as read-only statistics (no longer the chart-range control)

### Requirement: State-timeline detail renders the full selected window
The public-IP/border detail timeline SHALL render the entire selected range, not only the most recent capped slice of raw probes. For a high-frequency monitor, the timeline SHALL still show the whole window's state history.

#### Scenario: Full window shown for a fast monitor
- **WHEN** a public-IP monitor probing frequently is viewed over a multi-hour window
- **THEN** the timeline shows the whole window's IP history, not just the most recent ~15 minutes

### Requirement: Retention is shown and configurable in the UI
The add-monitor form SHALL collect a monitor's retention in hours, and the monitor detail page SHALL surface the configured retention.

#### Scenario: Set retention when creating a monitor
- **WHEN** a user creates a monitor and enters a retention in hours
- **THEN** the monitor is created with that retention

#### Scenario: Retention shown on the detail page
- **WHEN** a user views a monitor's detail page
- **THEN** the monitor's configured retention is displayed

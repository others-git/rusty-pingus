## ADDED Requirements

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

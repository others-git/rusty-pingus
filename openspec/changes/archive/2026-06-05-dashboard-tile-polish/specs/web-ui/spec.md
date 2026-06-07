## ADDED Requirements

### Requirement: Monitor tile is a clickable link with non-overlapping controls
Each dashboard monitor tile SHALL be a single clickable link to that monitor's detail page — clicking anywhere on the tile that is not an interactive control SHALL navigate to the detail page. The per-tile controls (pause/resume and delete) SHALL be positioned so they do not overlap the status badge; they SHALL remain independently clickable (activating a control SHALL NOT navigate to the detail page) and MAY be revealed on hover.

#### Scenario: Clicking the tile opens the detail page
- **WHEN** a user clicks anywhere on a monitor tile other than a control button
- **THEN** the browser navigates to that monitor's detail page

#### Scenario: Controls do not overlap the status badge
- **WHEN** a monitor tile renders (and its controls are revealed)
- **THEN** the pause/resume and delete controls are positioned away from the status badge and do not cover it

#### Scenario: Activating a control does not navigate
- **WHEN** a user clicks the pause/resume or delete control on a tile
- **THEN** that action runs (toggle or delete-confirm) and the tile does not navigate to the detail page

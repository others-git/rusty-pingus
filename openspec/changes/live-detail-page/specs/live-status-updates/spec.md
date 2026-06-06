## ADDED Requirements

### Requirement: Detail pages update live with polling fallback
The monitor detail page SHALL subscribe to the status stream (`GET /api/monitors/stream`) and update in place as status updates arrive for the monitor it is displaying, without a page reload. A periodic poll SHALL be retained as a fallback so the detail page stays current when the stream is unavailable or interrupted. Updates for other monitors SHALL be ignored by the page.

#### Scenario: Detail page reflects a new probe without reload
- **WHEN** a user is on a monitor's detail page with a live connection and that monitor records a new probe result
- **THEN** the page's status, detail, and metrics update in place within about a second, without a reload

#### Scenario: Newly available data fills in
- **WHEN** a detail page is opened for a monitor that has no data in the active window yet, and probe results subsequently arrive
- **THEN** the page populates its metrics and chart from the arriving data without the user reloading

#### Scenario: Updates for other monitors are ignored
- **WHEN** the stream delivers an update for a monitor other than the one being viewed
- **THEN** the detail page does not change

#### Scenario: Falls back to polling
- **WHEN** the live stream is unavailable or drops
- **THEN** the detail page still stays current via its periodic poll

## REMOVED Requirements

### Requirement: Detail pages do not auto-update
**Reason**: Static detail pages get stuck on a stale or empty snapshot (e.g. a newly added/renamed monitor, or just after a restart) while the SSE-driven dashboard shows the monitor as healthy, which reads as a bug. Detail pages now update live.
**Migration**: None for users. The detail page now subscribes to the existing `GET /api/monitors/stream` feed (with poll fallback); no API or data changes are required. See the new requirement "Detail pages update live with polling fallback".

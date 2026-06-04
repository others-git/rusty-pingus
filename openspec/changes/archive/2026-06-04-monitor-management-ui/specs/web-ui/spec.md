## MODIFIED Requirements

### Requirement: Dashboard overview page
The system SHALL serve a dashboard page at `/` showing the current status of all configured monitors including name, protocol, endpoint, current status (up/down), last check time, and uptime percentage for the last 24 hours. The page SHALL include a sticky header displaying a global summary (total monitors, count up, count down), and monitor cards SHALL be rendered using Alpine.js reactive state populated from the `/api/monitors` JSON endpoint. Each card SHALL display a Font Awesome protocol icon, an animated pulse indicator for `up` status, a styled status badge, the response time in ms, and the 24h uptime percentage. Each monitor card SHALL include a delete button that, on confirmation, calls `DELETE /api/monitors/:name` and removes the card from the list. The header SHALL include an "Add Monitor" button that opens the add-monitor modal.

#### Scenario: All monitors shown
- **WHEN** a user opens the dashboard
- **THEN** every configured monitor appears as a dark card with its current status indicated visually (emerald pulse for up, red badge for down, amber for pending)

#### Scenario: Last check time displayed
- **WHEN** a monitor has completed at least one probe
- **THEN** the dashboard shows the time of the most recent probe result for that monitor, formatted as a relative time (e.g., "2 minutes ago")

#### Scenario: No probes yet
- **WHEN** a monitor has not yet completed any probes
- **THEN** the dashboard shows that monitor with an amber "pending" badge and no response time

#### Scenario: Global summary header shows counts
- **WHEN** the dashboard loads
- **THEN** a sticky header bar displays "X / Y Monitors Up" and the total count of monitors in "down" state

#### Scenario: Delete monitor with confirmation
- **WHEN** a user clicks the delete button on a monitor card
- **THEN** a confirmation dialog appears; on confirm, `DELETE /api/monitors/:name` is called and the card is removed from the dashboard without a page reload

#### Scenario: Add Monitor button opens modal
- **WHEN** a user clicks the "Add Monitor" button in the header
- **THEN** the add-monitor modal opens

### Requirement: Add monitor modal form
The system SHALL provide a modal form for adding new monitors, accessible from the dashboard header. The form SHALL support HTTP, TCP, and ICMP protocols via tabs or a protocol selector. The form SHALL validate inputs inline and display errors next to the relevant field. On success, the modal SHALL close and the new monitor SHALL appear in the dashboard.

#### Scenario: HTTP monitor added successfully
- **WHEN** a user fills in the HTTP form (name, URL, interval) and submits
- **THEN** the monitor appears in the dashboard and the modal closes

#### Scenario: Validation error shown inline
- **WHEN** a user submits the form with an invalid URL
- **THEN** the URL field shows an error message and the form does not submit

#### Scenario: Duplicate name rejected
- **WHEN** a user submits a name that already exists
- **THEN** the name field shows "A monitor with this name already exists" and the form does not submit

#### Scenario: Protocol selector changes visible fields
- **WHEN** a user selects "TCP" in the protocol selector
- **THEN** host and port fields are shown and the URL field is hidden

#### Scenario: Modal dismissible
- **WHEN** a user clicks outside the modal or presses Escape
- **THEN** the modal closes without saving

### Requirement: Dashboard auto-refresh
The system SHALL poll the `/api/monitors` endpoint every 30 seconds using Alpine.js reactive state and update the dashboard display without a full page reload. A visual "last updated" timestamp SHALL be shown and updated on each successful poll.

#### Scenario: Status updates without reload
- **WHEN** a monitor changes status between polls
- **THEN** the Alpine.js reactive state update causes the card to re-render with the new status at the next polling interval without a browser refresh

#### Scenario: Last updated timestamp refreshes
- **WHEN** a successful poll completes
- **THEN** the "Last updated" display in the header updates to reflect the current time

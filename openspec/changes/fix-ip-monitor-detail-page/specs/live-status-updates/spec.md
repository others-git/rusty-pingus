## ADDED Requirements

### Requirement: State-timeline detail pages do not auto-reload the chart per probe
On a state-timeline monitor detail page (public-IP, border), the page SHALL update the header's current value (e.g. current public IP, fault class) live as probes arrive, but SHALL NOT reload or re-render the chart/timeline on every probe. The chart SHALL reload only on user action (selecting a window, zoom/pan, or an explicit refresh) or when the tracked value actually changes. This prevents continuous re-rendering ("flashing") for monitors whose value rarely changes.

#### Scenario: Header updates live without the chart flashing
- **WHEN** a public-IP monitor's detail page is open and successive probes return the same IP
- **THEN** the header reflects the latest check, and the IP timeline is not re-rendered for each probe (no flashing)

#### Scenario: Chart updates when the tracked value changes
- **WHEN** a public-IP monitor's observed IP changes while its detail page is open
- **THEN** the timeline updates to reflect the new value

#### Scenario: Response-chart monitors still update live
- **WHEN** a response-time monitor (e.g. ICMP/HTTP/TCP) detail page is open
- **THEN** its live-update behavior is unchanged by this requirement (it applies only to state-timeline monitors)

## ADDED Requirements

### Requirement: Traceroute privilege/unavailability is surfaced distinctly
When a `traceroute` monitor's latest probe failed because its raw socket could not be opened (a privilege/socket failure such as missing CAP_NET_RAW), the dashboard and the traceroute detail view SHALL present a distinct "unavailable — requires elevated privileges" state with a short explanation, visually separate from a normal network "down". A traceroute that fails for ordinary network reasons (timeout/unreachable) SHALL keep its normal down presentation.

#### Scenario: Privilege failure shown as unavailable, not plain down
- **WHEN** a traceroute monitor's latest probe failed due to a privilege/socket error
- **THEN** its dashboard card and detail page show a distinct "unavailable — needs elevated privileges (CAP_NET_RAW)" state with a brief explanation, not a bare "down"

#### Scenario: Ordinary failure unaffected
- **WHEN** a traceroute monitor fails for a normal network reason (e.g. timeout)
- **THEN** it is shown with the usual down presentation, not the privilege/unavailable state

#### Scenario: State clears when the monitor can run
- **WHEN** a previously privilege-failed traceroute monitor later probes successfully
- **THEN** the unavailable state is no longer shown

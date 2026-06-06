## ADDED Requirements

### Requirement: Traceroute probe execution
The probe engine SHALL execute a `traceroute` monitor by performing a TTL-walked ICMP traceroute to the target and producing a multi-hop result (an ordered list of hops with address, reachability, and RTT statistics) plus an overall status. The traceroute probe SHALL run under the same isolation guarantees as other probes — a failure or panic in one probe SHALL NOT affect others.

#### Scenario: Successful traceroute yields a multi-hop result
- **WHEN** a traceroute monitor runs against a reachable target
- **THEN** the probe produces an ordered list of hops up to the destination and an overall "up" status

#### Scenario: Unopenable raw socket fails the monitor cleanly
- **WHEN** the raw socket required for traceroute cannot be opened (e.g. insufficient privileges)
- **THEN** the probe records a down result with a descriptive reason rather than crashing or stalling the engine

#### Scenario: Traceroute writes a dashboard summary alongside hop data
- **WHEN** a traceroute run completes
- **THEN** a lightweight summary (status, destination RTT, and a short detail) is recorded so the monitor appears on the dashboard and live stream like other monitors, while the full per-hop data is stored separately

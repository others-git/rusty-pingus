## Purpose

Defines how the probe engine executes HTTP, TCP, and ICMP probes, the structure of probe results, and the isolation guarantees between monitors.

## Requirements

### Requirement: HTTP probe execution
The system SHALL execute HTTP probes using a non-blocking async HTTP client, recording status code, response time, and success/failure.

#### Scenario: Successful HTTP probe
- **WHEN** an HTTP probe receives a response with the expected status code within the timeout
- **THEN** the result is recorded as `up` with the measured response time in milliseconds

#### Scenario: HTTP probe timeout
- **WHEN** an HTTP probe does not receive a response within the configured timeout
- **THEN** the result is recorded as `down` with reason `timeout`

#### Scenario: HTTP probe connection refused
- **WHEN** an HTTP probe target actively refuses the connection
- **THEN** the result is recorded as `down` with reason `connection_refused`

#### Scenario: HTTP probe unexpected status code
- **WHEN** an HTTP probe receives a response but the status code does not match the expected value
- **THEN** the result is recorded as `down` with the actual status code captured in the result

#### Scenario: HTTPS with TLS error
- **WHEN** an HTTPS probe encounters a TLS certificate error
- **THEN** the result is recorded as `down` with reason `tls_error`

### Requirement: TCP probe execution
The system SHALL execute TCP probes by attempting a full TCP handshake to the configured host and port.

#### Scenario: Successful TCP probe
- **WHEN** a TCP probe completes a handshake within the timeout
- **THEN** the result is recorded as `up` with the measured connection time in milliseconds

#### Scenario: TCP probe timeout
- **WHEN** a TCP probe does not complete a handshake within the configured timeout
- **THEN** the result is recorded as `down` with reason `timeout`

#### Scenario: TCP probe host not found
- **WHEN** a TCP probe target hostname cannot be resolved
- **THEN** the result is recorded as `down` with reason `dns_error`

### Requirement: ICMP probe execution
The system SHALL execute ICMP echo request/reply probes by sending a configurable number of echo requests per probe cycle (the monitor's packet count), each bounded by the monitor's timeout. The monitor SHALL be recorded as `up` if at least one echo reply is received, and as `down` only when all echo requests are lost. For an `up` result the recorded round-trip time SHALL be the best (lowest) RTT among the replies received. Packet loss SHALL be reflected in the failure reason: a full loss SHALL report the replies-received over packets-sent counts, and a partial loss while still `up` SHALL note the loss informationally without changing the status to `down`.

#### Scenario: All replies received
- **WHEN** every ICMP echo request in a cycle receives a reply within the timeout
- **THEN** the result is recorded as `up` with the lowest measured round-trip time in milliseconds and no failure reason

#### Scenario: Partial loss still up
- **WHEN** at least one but not all echo requests receive a reply within the timeout
- **THEN** the result is recorded as `up` with the best round-trip time, and the failure reason notes the partial loss (replies received over packets sent)

#### Scenario: All requests lost
- **WHEN** no echo reply is received for any request within the timeout
- **THEN** the result is recorded as `down` with a reason indicating no reply and the replies-received over packets-sent counts

#### Scenario: Single-packet configuration
- **WHEN** an ICMP monitor is configured with a packet count of 1
- **THEN** the probe sends a single echo and the result is `up` on a reply or `down` on no reply, matching basic single-shot behavior

### Requirement: Probe result structure
Each probe execution SHALL produce a structured result containing: monitor name, timestamp (UTC), protocol, status (`up`/`down`), response time (ms, nullable), and failure reason (nullable).

#### Scenario: Up result fields
- **WHEN** a probe succeeds
- **THEN** the result contains `status = "up"`, a non-null `response_time_ms`, and a null `failure_reason`

#### Scenario: Down result fields
- **WHEN** a probe fails
- **THEN** the result contains `status = "down"`, a null or absent `response_time_ms`, and a non-null `failure_reason` string

### Requirement: Probe isolation
The system SHALL ensure that a slow or hung probe for one monitor does not delay probes for other monitors.

#### Scenario: Slow probe does not block others
- **WHEN** monitor A's probe is waiting for a timeout (e.g., 30 seconds)
- **THEN** monitor B continues to execute its probes on schedule without waiting for A

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

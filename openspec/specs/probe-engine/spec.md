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
The system SHALL execute ICMP echo request/reply probes, recording round-trip time and packet loss.

#### Scenario: Successful ICMP probe
- **WHEN** an ICMP echo reply is received within the timeout
- **THEN** the result is recorded as `up` with the measured round-trip time in milliseconds

#### Scenario: ICMP probe no reply
- **WHEN** no ICMP echo reply is received within the timeout
- **THEN** the result is recorded as `down` with reason `no_reply`

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

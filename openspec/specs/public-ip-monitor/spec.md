# Public-IP Monitor Spec

## Purpose

Defines the `publicip` monitor type, which periodically queries an external IP-echo service to record the host's public IP and surface changes to it over time.

## Requirements

### Requirement: Public-IP monitor type
The system SHALL support a monitor type (`protocol = "publicip"`) that periodically queries an external IP-echo service and records the host's public IP. It SHALL accept an optional service URL (defaulting to a known IP-echo service with a fallback) plus the standard name/interval/timeout. The check is **up** when the service responds successfully with a parseable IP address, which SHALL be recorded as the probe's detail; otherwise it is **down** with a failure reason.

#### Scenario: Records the public IP when reachable
- **WHEN** a public-IP monitor runs and the IP service returns a valid address
- **THEN** the probe is `up`, and the returned IP is recorded as the probe's detail

#### Scenario: Falls back to a secondary service
- **WHEN** the primary IP-echo service fails to respond
- **THEN** the monitor tries the configured fallback service before reporting down

#### Scenario: Down when no IP can be obtained
- **WHEN** the service is unreachable or returns an unparseable body
- **THEN** the probe is `down` with a failure reason

### Requirement: Public-IP change detection
The system SHALL detect when a public-IP monitor's observed IP differs from its most recently recorded IP and surface that change in the probe's detail so it is visible in the monitor's history.

#### Scenario: IP change is surfaced
- **WHEN** a public-IP check returns an address different from the previously recorded one
- **THEN** the new IP is recorded and the detail indicates that the IP changed (including the previous value)

#### Scenario: Unchanged IP is not flagged as a change
- **WHEN** consecutive checks return the same IP
- **THEN** the detail records the IP without indicating a change

## ADDED Requirements

### Requirement: Border monitor type
The system SHALL support a monitor type (`protocol = "border"`) that localizes connectivity faults by ICMP-probing the local gateway and an upstream reference on each cycle. It SHALL classify the outcome and record that classification as the probe's detail:
- gateway reachable AND upstream reachable → status `up`, detail indicates OK (with both round-trip times);
- gateway reachable AND upstream unreachable → status `down`, detail indicates the fault is upstream (ISP/internet);
- gateway unreachable → status `down`, detail indicates the fault is local (LAN/gateway).
Overall status SHALL follow upstream reachability, and the recorded response time SHALL be the upstream round-trip time.

#### Scenario: Everything reachable
- **WHEN** both the gateway and the upstream reference respond
- **THEN** the probe is `up` and the detail indicates OK with both round-trip times

#### Scenario: ISP/internet down, LAN up
- **WHEN** the gateway responds but the upstream reference does not
- **THEN** the probe is `down` and the detail localizes the fault as upstream (ISP)

#### Scenario: Local gateway down
- **WHEN** the gateway does not respond
- **THEN** the probe is `down` and the detail localizes the fault as local (LAN/gateway)

### Requirement: Border monitor targets
The system SHALL let a border monitor specify the gateway and upstream targets, and SHALL attempt to auto-detect the local default gateway when one is not configured. The upstream reference SHALL default to a well-known address when unspecified. When the gateway is neither configured nor detectable, the monitor SHALL report a clear failure rather than guessing.

#### Scenario: Configured targets are used
- **WHEN** a border monitor specifies a gateway and/or upstream
- **THEN** those addresses are probed

#### Scenario: Gateway auto-detected
- **WHEN** no gateway is configured and the default gateway can be detected
- **THEN** the detected gateway is probed

#### Scenario: Gateway unavailable
- **WHEN** no gateway is configured and none can be detected
- **THEN** the probe reports `down` with a failure reason indicating the gateway is not configured/detected

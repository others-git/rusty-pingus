## MODIFIED Requirements

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

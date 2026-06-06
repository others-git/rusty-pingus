## ADDED Requirements

### Requirement: Traceroute probe records the per-hop path
A `traceroute` monitor SHALL probe the route to its configured target by sending ICMP Echo Requests with increasing IP TTL (from 1 up to a configured maximum), recording for each hop its responding address (or unreachable), and the round-trip time. The walk SHALL terminate when the destination replies or the maximum hop count is reached.

#### Scenario: Intermediate hop reports its address and RTT
- **WHEN** a probe with TTL = N elicits an ICMP Time-Exceeded from an intermediate router
- **THEN** hop N is recorded with that router's address and the measured round-trip time

#### Scenario: Destination reached ends the walk
- **WHEN** a probe elicits an Echo-Reply from the target address
- **THEN** that hop is recorded as the destination, marked reached, and no higher-TTL probes are sent for the run

#### Scenario: Non-responding hop is recorded as unreachable
- **WHEN** a hop produces no reply within the configured timeout
- **THEN** the hop is recorded as unreachable (no address, no RTT) without aborting the rest of the run

#### Scenario: Maximum hop count bounds the run
- **WHEN** the destination has not replied by the configured maximum hop count
- **THEN** the run stops at the maximum and is recorded as not having reached the destination

### Requirement: Per-run min/avg/max from multiple queries per hop
A `traceroute` monitor SHALL send a configurable number of queries per hop within a run and SHALL compute the minimum, average, and maximum round-trip time per hop for that run from the responding queries.

#### Scenario: Min/avg/max computed from responding queries
- **WHEN** a hop responds to several of the run's queries
- **THEN** the hop's recorded min, avg, and max RTT are derived from those responding queries

#### Scenario: Partial loss at a hop is represented
- **WHEN** a hop responds to some but not all of its queries
- **THEN** the hop's loss for the run reflects the missing replies while min/avg/max use the replies received

### Requirement: Hops are queryable over a requested time range
The system SHALL expose the traceroute data for a monitor over an explicit `[from, to]` time range, returning per-hop aggregates (latest address, reachability, and min/avg/max RTT) computed across the runs in that range.

#### Scenario: Aggregation over a sub-range of retained data
- **WHEN** a client requests a monitor's hops for a time range within the retained data
- **THEN** the response contains, per hop position, the aggregated reachability and min/avg/max RTT over the runs in that range

#### Scenario: Empty range yields no hops
- **WHEN** a client requests a range that contains no runs
- **THEN** the response is empty rather than an error

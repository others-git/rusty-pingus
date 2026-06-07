## MODIFIED Requirements

### Requirement: Per-hop latency graph
The traceroute detail page SHALL visualize per-hop latency as a single continuous min–max ribbon aligned to the hop rows: one filled band whose edges follow each hop's minimum and maximum RTT, connected vertically across hops so it forms one continuous shape, with the per-hop averages overlaid as a line of markers. Relative latency and where the min–max spread widens across the path SHALL be readable at a glance. A non-responding hop SHALL break the ribbon (and the average line) rather than being drawn as zero latency.

#### Scenario: Latency shown as a continuous min–max ribbon
- **WHEN** the hop table renders
- **THEN** the min–max range is drawn as one continuous filled ribbon across the hops, with the per-hop averages overlaid as a connected line of markers

#### Scenario: Non-responding hop breaks the ribbon
- **WHEN** a hop did not respond over the active range
- **THEN** the ribbon (and the average line) break at that hop rather than pinching to zero latency

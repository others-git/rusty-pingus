## Why

The ICMP probe sends exactly one echo request (`PingSequence(0)`) per cycle and marks the monitor **down** the moment that single packet times out (`src/probe/icmp.rs:47`). ICMP is inherently lossy and is commonly rate-limited, deprioritized, or tunneled unreliably — especially behind a VPN. A host that is plainly reachable can drop or delay an individual echo, and the current probe reads that single loss as a full outage. This produces false "down" flips and noisy uptime history that does not reflect reality.

Both canonical specs already gesture at richer behavior the code never delivered: `monitor-config` says ICMP monitors have a configurable **packet count**, and `probe-engine` says ICMP probes record **round-trip time and packet loss** — yet the implementation is single-shot with no count field and no loss measurement. This change makes the implementation match and tighten those specs.

## What Changes

- The ICMP probe sends **multiple echo requests** per cycle (default 3) instead of one, using distinct sequence numbers, each bounded by the per-monitor `timeout_ms`.
- Up/down rule becomes **tolerant**: the monitor is **up if at least one** echo reply is received; it is **down only when all** echoes are lost. The reported `response_time_ms` is the **best (lowest) RTT** among the replies.
- **Packet loss is surfaced** in `failure_reason`: on a full loss the reason carries the replies/sent counts (e.g. `no_reply (0/3 replies)`); on a partial-loss-but-up result the loss is noted informationally without flipping the monitor down.
- A new optional ICMP config field **`count`** (packets per cycle) is honored, with a sensible default and a lower bound of 1 (1 = current single-shot behavior).
- Non-loss errors (DNS failure, privilege error, socket error) keep their existing distinct reasons and are unaffected by the retry logic.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `probe-engine`: ICMP probe execution changes from single-echo pass/fail to multi-echo with loss tolerance (up on any reply, best-RTT, loss surfaced in the reason).
- `monitor-config`: the ICMP monitor's `count` (packets-per-cycle) field is honored with a documented default and minimum.

## Impact

- **Code:** `src/probe/icmp.rs` (multi-echo loop, loss accounting, reason text); `src/monitors/mod.rs` (`IcmpMonitorConfig` + `RawIcmpMonitorConfig` gain `count`, default helper); default-config example comment may mention `count`.
- **Behavior:** existing ICMP monitors become more stable; no DB schema change (loss rides in the existing `failure_reason` field; `ProbeResult` is unchanged).
- **Compatibility:** configs without `count` keep working via the default; `count = 1` reproduces today's behavior. Slightly more ICMP traffic per cycle (default 3 packets vs 1).

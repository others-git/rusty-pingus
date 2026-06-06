## 1. Config: ICMP packet count

- [x] 1.1 Add `count: u32` to `IcmpMonitorConfig` and an optional `count: Option<u32>` to `RawIcmpMonitorConfig` in `src/monitors/mod.rs`
- [x] 1.2 Add `fn default_icmp_count() -> u32 { 3 }`; in the `From<RawIcmpMonitorConfig>` impl set `count: r.count.unwrap_or_else(default_icmp_count).max(1)`
- [x] 1.3 (If the default-config generator lists ICMP fields) add a commented `# count = 3` example for ICMP monitors

## 2. Probe: multi-echo with loss tolerance

- [x] 2.1 In `src/probe/icmp.rs::run`, keep DNS resolution / client construction / privilege handling unchanged (they short-circuit before the echo loop with their existing reasons)
- [x] 2.2 Replace the single `pinger.ping(PingSequence(0), &[])` with a loop over sequence numbers `0..cfg.count`, each bounded by `cfg.timeout_ms`; tally `received`, track `min_rtt` (lowest successful RTT), and remember the last non-timeout error string
- [x] 2.3 Decide the result: `received >= 1` → `ProbeResult::up` with `response_time_ms = min_rtt`; `received == 0` → `ProbeResult::down`
- [x] 2.4 Set `failure_reason`: clean (all replied) → none; partial loss but up → `partial_loss ({received}/{count} replies)` with status still `up`; full loss → `no_reply ({received}/{count} replies)` (or the preserved non-timeout error if every echo errored non-timeout)
- [x] 2.5 Keep `debug!`/`warn!` logging meaningful (e.g. log received/sent and best RTT)

## 3. Tests

- [x] 3.1 Unit-test the decision/tally logic: all-replied → up + min RTT + no reason; partial → up + partial_loss reason; all-lost → down + no_reply with counts (factor the pure decision out of the network call if needed to make it testable)
- [x] 3.2 Confirm `count` defaulting/clamping: missing → 3, `0` → 1, explicit value respected (config deserialization test)
- [x] 3.3 Ensure existing ICMP/probe tests still pass (`cargo test`)

## 4. Verification

- [x] 4.1 `cargo build`, `cargo clippy -- -D warnings`, and `cargo test` pass
- [ ] 4.2 (Manual, environmental) With an ICMP monitor pointed at a reachable host behind the VPN, confirm the monitor stays `up` across cycles where individual packets drop, and that partial loss is visible in the reason; confirm a genuinely unreachable host still reports `down` with the loss counts
- [x] 4.3 Update `README.md` / config docs to mention the ICMP `count` field and the "up if any reply" behavior

## Context

`src/probe/icmp.rs::run` resolves the host to an `IpAddr`, builds a `surge_ping` client, creates a pinger, and sends **one** echo (`pinger.ping(PingSequence(0), &[])`) bounded by `cfg.timeout_ms`. On `Ok` it returns `ProbeResult::up` with the elapsed RTT; on a timeout error it returns `ProbeResult::down` with reason `no_reply`, and on other errors `error: {e}`. DNS failure returns `dns_error`; client construction failure returns `privilege_error`.

`IcmpMonitorConfig` ( `src/monitors/mod.rs` ) holds `name, host, interval_ms, timeout_ms`, deserialized from `RawIcmpMonitorConfig` via a `From` impl that uses `resolve_ms` for the `_ms`/legacy `_secs` keys. `ProbeResult` (`src/probe/mod.rs`) has fields `response_time_ms: Option<u64>` and `failure_reason: Option<String>` — there is **no `detail` field**, and `up` sets `failure_reason = None`.

`surge_ping` reuses one `Client`/`pinger` to send multiple sequences; each `ping(seq, payload)` awaits one reply bounded by the pinger's `timeout`.

## Goals / Non-Goals

**Goals:**
- Stop false "down" flips caused by single dropped ICMP echoes over VPN/lossy links.
- Report the best observed RTT and make packet loss visible without a schema change.
- Keep the change isolated to the ICMP probe + its config; no DB, API, or other-protocol changes.

**Non-Goals:**
- TCP/HTTP fallback reachability checks (different semantics; out of scope).
- A configurable loss *threshold* policy (we chose "any reply = up"; threshold mode is explicitly not built).
- Adding a `detail` column to `ProbeResult`/DB (loss rides in `failure_reason`).
- Per-packet RTT history or jitter/percentile stats.

## Decisions

### 1. Multi-echo loop, "any reply = up"
`run` sends `count` echoes with sequence numbers `0..count` on the same pinger, each bounded by `cfg.timeout_ms`. It tallies `received` and tracks the **minimum** successful RTT. After the loop:
- `received >= 1` → `ProbeResult::up` with `response_time_ms = min_rtt`.
- `received == 0` → `ProbeResult::down`.

Echoes are sent **sequentially** (simplest; matches the per-cycle cadence and avoids burst). Worst-case added latency for a fully-down host is `count * timeout_ms`; acceptable given default `count = 3` and that down detection isn't latency-critical. (Sequential vs concurrent noted as a trade-off below.)

### 2. Loss surfaced in `failure_reason`
Because `up` conventionally has `failure_reason = None`, but we want partial loss visible:
- Full loss (down): `failure_reason = "no_reply (0/{count} replies)"`.
- Partial loss (still up): set `failure_reason = Some("partial_loss ({received}/{count} replies)")` while `status = "up"`. This is an intentional, documented exception to "up ⇒ failure_reason is null" — the field doubles as a health note and the UI already renders it. Clean (zero loss): `failure_reason = None`.

This keeps the `probe-engine` "Up result fields" scenario true for the clean case and adds a partial-loss nuance rather than contradicting it.

### 3. `count` config field
Add `count: u32` to `IcmpMonitorConfig` and an optional `count` to `RawIcmpMonitorConfig`. Default via `default_icmp_count() -> 3`. Clamp to a minimum of 1 in the `From` impl (`r.count.unwrap_or(3).max(1)`) so `count = 0`/missing never yields zero packets. `count = 1` exactly reproduces today's single-shot behavior. No legacy `_secs`-style alias is needed (new field).

### 4. Non-loss errors unchanged
DNS resolution failure (`dns_error`), client/socket construction failure (`privilege_error`), and other non-timeout `surge_ping` errors keep their current reasons and short-circuit before/within the loop. Only **timeouts / no-reply** participate in the loss tally. A non-timeout error on an individual echo is treated as that echo failing (counts as a loss) so a transient error mid-loop doesn't abort an otherwise-up probe; if *every* echo errors non-timeout, the last error string is preserved in the reason.

## Risks / Trade-offs

- **More ICMP traffic:** default 3 packets/cycle vs 1. Negligible, and `count` is tunable down to 1.
- **Slower down-confirmation:** a fully-unreachable host now takes up to `count * timeout_ms` per cycle (sequential sends). Acceptable; could switch to concurrent sends later if it matters.
- **`failure_reason` on up results:** the partial-loss note technically loosens the "up ⇒ null reason" invariant. Mitigated by only populating it on actual partial loss and documenting it in the spec scenario; consumers that treat any non-null reason as "down" must key off `status`, not `failure_reason` (they already should).
- **Verification is partly environmental:** reproducing VPN loss deterministically is hard; unit-level tests cover the tally/decision logic, and the real-world VPN case is a manual check.

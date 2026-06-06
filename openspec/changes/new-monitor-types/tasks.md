## 1. Per-probe detail field

- [x] 1.1 Add `migrations/004_detail.sql`: `ALTER TABLE probe_results ADD COLUMN detail TEXT`
- [x] 1.2 Add `detail: Option<String>` to `ProbeResult` (`src/probe/mod.rs`); add a helper to attach detail (e.g. `with_detail`) to the `up`/`down` constructors
- [x] 1.3 Persist `detail` in `db::insert_result`; include it in `get_history` (`HistoryRow`) and `get_latest_status`/`get_current_status` (`CurrentStatus`) + the `/api/monitors` and history JSON

## 2. Public-IP monitor

- [x] 2.1 Add `PublicIpMonitorConfig { name, url: Option<String>, interval_ms, timeout_ms }` and a `PublicIp` variant to `MonitorConfig`; update `name()`/`interval_ms()`/`protocol()` ("publicip") / `endpoint()` and the legacy-aware deserialization defaults
- [x] 2.2 Add `src/probe/publicip.rs`: GET the service (default `https://checkip.amazonaws.com`, fallback `https://icanhazip.com`), parse/validate the body as an `IpAddr`; up → record IP in `detail` + response time; down → failure reason
- [x] 2.3 Change detection: look up the monitor's most recent prior detail; if the IP changed, set detail to include the previous value
- [x] 2.4 Dispatch `PublicIp` in `scheduler::run_probe`

## 3. Border monitor

- [x] 3.1 Refactor `probe::icmp` to expose a reusable `ping_once(host, timeout) -> Result<rtt_ms>` helper used by both ICMP and border probes
- [x] 3.2 Add `BorderMonitorConfig { name, gateway: Option<String>, upstream: String (default 1.1.1.1), interval_ms, timeout_ms }` and a `Border` variant; update the enum methods (`protocol()` "border", `endpoint()` = "gateway → upstream")
- [x] 3.3 Add `src/probe/border.rs`: best-effort default-gateway auto-detect (Linux `/proc/net/route`, Windows, macOS) used when `gateway` is unset; ping gateway + upstream; classify `ok` / `isp_down` / `lan_down`; status = upstream reachability; response time = upstream RTT; classification + RTTs in `detail`; clear failure if no gateway available
- [x] 3.4 Dispatch `Border` in `scheduler::run_probe`

## 4. API validation

- [x] 4.1 In `src/api/mod.rs` `validate_monitor`, handle `publicip` (optional `url` must be http(s)) and `border` (`gateway` if set + `upstream` must be valid IP/host); keep name rules

## 5. UI

- [x] 5.1 In `assets/index.html` + `assets/app.js`, add "Public IP" and "Border" protocol tabs to the add-monitor modal with their fields (publicip: optional URL; border: optional gateway, upstream); build the request body accordingly
- [x] 5.2 Surface `detail` on monitor cards / detail view when present (current public IP; border classification), without disrupting layout

## 6. Verification

- [x] 6.1 `cargo build` and `cargo clippy -- -D warnings` pass
- [x] 6.2 `cargo test` passes; add tests for the IP parse/up-down path and the border classification logic (ok / isp_down / lan_down), and that `detail` round-trips through insert→history
- [x] 6.3 Smoke test: add a public-IP monitor → confirm it records an IP (and flags a change if it changes); add a border monitor (configured gateway + upstream) → confirm classification; confirm both appear in the add-monitor UI and their detail shows on the dashboard
- [x] 6.4 Update `README.md`: document the `publicip` and `border` monitor types (config fields, behavior, border's ICMP-privilege requirement)

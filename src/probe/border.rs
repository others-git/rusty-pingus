use std::time::Duration;
use tracing::{debug, warn};

use crate::monitors::BorderMonitorConfig;
use super::icmp::{ping_once, PingError};
use super::ProbeResult;

/// Localize a connectivity fault by ICMP-probing the local gateway and an
/// upstream reference, then classifying the outcome:
/// - gateway up + upstream up → `up` (`ok`)
/// - gateway up + upstream down → `down` (`isp_down`)
/// - gateway down → `down` (`lan_down`)
///
/// Overall status follows upstream reachability and the recorded response time is
/// the upstream RTT. The classification and both RTTs ride in `detail`.
pub async fn run(cfg: &BorderMonitorConfig) -> ProbeResult {
    let timeout = Duration::from_millis(cfg.timeout_ms);

    // Resolve the gateway: configured value, else best-effort auto-detection.
    let gateway = match cfg.gateway.clone().or_else(detect_default_gateway) {
        Some(g) => g,
        None => {
            let endpoint = format!("auto → {}", cfg.upstream);
            return ProbeResult::down(&cfg.name, "border", &endpoint, "gateway_not_configured")
                .with_detail("lan_down — gateway not configured or detected".to_string());
        }
    };
    let endpoint = format!("{} → {}", gateway, cfg.upstream);

    let gw = ping_once(&gateway, timeout).await;
    let up = ping_once(&cfg.upstream, timeout).await;

    // A missing raw-socket privilege isn't a connectivity fault — surface it as
    // ICMP does rather than misclassifying it as LAN down.
    if matches!(gw, Err(PingError::Privilege)) || matches!(up, Err(PingError::Privilege)) {
        warn!(monitor = %cfg.name, "Border probe socket error — check CAP_NET_RAW privileges");
        return ProbeResult::down(&cfg.name, "border", &endpoint, "privilege_error");
    }

    let gw_rtt = gw.as_ref().ok().copied();
    let up_rtt = up.as_ref().ok().copied();
    debug!(
        monitor = %cfg.name, gateway = %gateway, upstream = %cfg.upstream,
        gateway_ok = gw.is_ok(), upstream_ok = up.is_ok(), "Border probe"
    );

    classify_border(&cfg.name, &endpoint, gw.is_ok(), up.is_ok(), gw_rtt, up_rtt)
}

/// Pure classification of a border cycle, kept network-free for testing.
fn classify_border(
    name: &str,
    endpoint: &str,
    gateway_ok: bool,
    upstream_ok: bool,
    gw_rtt: Option<u64>,
    up_rtt: Option<u64>,
) -> ProbeResult {
    let fmt = |rtt: Option<u64>| rtt.map(|r| format!("{r}ms")).unwrap_or_else(|| "—".to_string());
    let rtts = format!("gw {}, upstream {}", fmt(gw_rtt), fmt(up_rtt));

    if !gateway_ok {
        ProbeResult::down(name, "border", endpoint, "lan_down")
            .with_detail(format!("lan_down — local gateway unreachable ({rtts})"))
    } else if upstream_ok {
        ProbeResult::up(name, "border", endpoint, up_rtt.unwrap_or(0))
            .with_detail(format!("ok ({rtts})"))
    } else {
        ProbeResult::down(name, "border", endpoint, "isp_down")
            .with_detail(format!("isp_down — gateway reachable, upstream unreachable ({rtts})"))
    }
}

/// Best-effort default-gateway detection. Returns `None` when it can't be
/// determined, in which case the monitor reports a clear failure rather than
/// guessing. Configuring `gateway` explicitly is the reliable path.
fn detect_default_gateway() -> Option<String> {
    #[cfg(target_os = "linux")]
    let detected = detect_gateway_linux();
    #[cfg(target_os = "macos")]
    let detected = detect_gateway_macos();
    #[cfg(target_os = "windows")]
    let detected = detect_gateway_windows();
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    let detected: Option<String> = None;
    detected
}

#[cfg(target_os = "linux")]
fn detect_gateway_linux() -> Option<String> {
    // /proc/net/route: the default route has destination 00000000; its gateway is
    // the IPv4 address in little-endian hex.
    let content = std::fs::read_to_string("/proc/net/route").ok()?;
    for line in content.lines().skip(1) {
        let mut fields = line.split_whitespace();
        let _iface = fields.next()?;
        let dest = fields.next()?;
        let gateway = fields.next()?;
        if dest == "00000000" {
            let raw = u32::from_str_radix(gateway, 16).ok()?;
            let o = raw.to_le_bytes();
            return Some(format!("{}.{}.{}.{}", o[0], o[1], o[2], o[3]));
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn detect_gateway_macos() -> Option<String> {
    let out = std::process::Command::new("route")
        .args(["-n", "get", "default"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        if let Some(rest) = line.trim().strip_prefix("gateway:") {
            let gw = rest.trim();
            if !gw.is_empty() {
                return Some(gw.to_string());
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn detect_gateway_windows() -> Option<String> {
    // `route print 0.0.0.0` lists default routes; the active default's line has
    // destination and netmask 0.0.0.0 and the gateway in the third column.
    let out = std::process::Command::new("route")
        .args(["print", "0.0.0.0"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() >= 3 && cols[0] == "0.0.0.0" && cols[1] == "0.0.0.0" {
            if cols[2].parse::<std::net::IpAddr>().is_ok() {
                return Some(cols[2].to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::classify_border;

    #[test]
    fn both_reachable_is_up_ok() {
        let r = classify_border("b", "gw → up", true, true, Some(2), Some(15));
        assert_eq!(r.status, "up");
        assert_eq!(r.response_time_ms, Some(15));
        assert!(r.detail.as_deref().unwrap().starts_with("ok "));
    }

    #[test]
    fn gateway_up_upstream_down_is_isp_down() {
        let r = classify_border("b", "gw → up", true, false, Some(2), None);
        assert_eq!(r.status, "down");
        assert_eq!(r.failure_reason.as_deref(), Some("isp_down"));
        assert!(r.detail.as_deref().unwrap().contains("isp_down"));
    }

    #[test]
    fn gateway_down_is_lan_down() {
        let r = classify_border("b", "gw → up", false, false, None, None);
        assert_eq!(r.status, "down");
        assert_eq!(r.failure_reason.as_deref(), Some("lan_down"));
        assert!(r.detail.as_deref().unwrap().contains("lan_down"));
    }

    #[test]
    fn gateway_down_upstream_up_still_lan_down() {
        // Gateway unreachable dominates the classification regardless of upstream.
        let r = classify_border("b", "gw → up", false, true, None, Some(15));
        assert_eq!(r.status, "down");
        assert_eq!(r.failure_reason.as_deref(), Some("lan_down"));
    }
}

use std::net::IpAddr;
use std::time::{Duration, Instant};
use surge_ping::{Client, Config, PingIdentifier, PingSequence, ICMP};
use tracing::{debug, warn};

use crate::monitors::IcmpMonitorConfig;
use super::ProbeResult;

pub fn check_privilege() -> bool {
    // Try creating a raw socket — succeeds only with CAP_NET_RAW or root
    std::net::UdpSocket::bind("0.0.0.0:0").is_ok()
        && Client::new(&Config::default()).is_ok()
}

/// Failure modes of a single ICMP echo, distinguished so callers can map them to
/// the right outcome (DNS/privilege short-circuit, timeout = packet loss, etc.).
#[derive(Debug)]
pub enum PingError {
    /// The host could not be resolved.
    Dns,
    /// A raw socket could not be created (missing CAP_NET_RAW / root).
    Privilege,
    /// No reply within the timeout (the echo was lost).
    NoReply,
    /// Any other error from the ICMP stack.
    Other(String),
}

/// Send a single ICMP echo request to `host`, returning the round-trip time in
/// milliseconds. Resolves the host, builds a one-shot client, and awaits one
/// reply bounded by `timeout`. Shared by the ICMP and border probes.
pub async fn ping_once(host: &str, timeout: Duration) -> Result<u64, PingError> {
    let ip: IpAddr = match host.parse() {
        Ok(ip) => ip,
        Err(_) => match tokio::net::lookup_host(format!("{host}:0")).await {
            Ok(mut addrs) => match addrs.next() {
                Some(addr) => addr.ip(),
                None => return Err(PingError::Dns),
            },
            Err(_) => return Err(PingError::Dns),
        },
    };

    let icmp_type = match ip {
        IpAddr::V4(_) => ICMP::V4,
        IpAddr::V6(_) => ICMP::V6,
    };

    let client = match Client::new(&Config::builder().kind(icmp_type).build()) {
        Ok(c) => c,
        Err(_) => return Err(PingError::Privilege),
    };

    let mut pinger = client.pinger(ip, PingIdentifier(rand_id())).await;
    pinger.timeout(timeout);

    let start = Instant::now();
    match pinger.ping(PingSequence(0), &[]).await {
        Ok(_) => Ok(start.elapsed().as_millis() as u64),
        Err(e) => {
            let s = e.to_string();
            if s.contains("timed out") || s.contains("timeout") {
                Err(PingError::NoReply)
            } else {
                Err(PingError::Other(format!("error: {e}")))
            }
        }
    }
}

pub async fn run(cfg: &IcmpMonitorConfig) -> ProbeResult {
    let endpoint = cfg.host.clone();
    let timeout = Duration::from_millis(cfg.timeout_ms);

    // Send `count` echoes sequentially via the shared `ping_once` helper. Tally
    // replies, track the best (lowest) RTT, and remember the last non-timeout
    // error. DNS / privilege failures are stable, so short-circuit on the first.
    let mut received: u32 = 0;
    let mut min_rtt: Option<u64> = None;
    let mut non_timeout_errors: u32 = 0;
    let mut last_non_timeout_error: Option<String> = None;

    for _ in 0..cfg.count {
        match ping_once(&cfg.host, timeout).await {
            Ok(rtt) => {
                received += 1;
                min_rtt = Some(min_rtt.map_or(rtt, |m| m.min(rtt)));
            }
            Err(PingError::Dns) => {
                return ProbeResult::down(&cfg.name, "icmp", &endpoint, "dns_error");
            }
            Err(PingError::Privilege) => {
                warn!(monitor = %cfg.name, "ICMP socket error — check CAP_NET_RAW privileges");
                return ProbeResult::down(&cfg.name, "icmp", &endpoint, "privilege_error");
            }
            // Timeout: a lost echo, tallied as loss without a non-timeout error.
            Err(PingError::NoReply) => {}
            Err(PingError::Other(msg)) => {
                non_timeout_errors += 1;
                last_non_timeout_error = Some(msg);
            }
        }
    }

    // Preserve a non-timeout error only when every echo failed that way.
    let preserved_error = if received == 0 && non_timeout_errors == cfg.count {
        last_non_timeout_error
    } else {
        None
    };

    let result = decide_icmp_result(&cfg.name, &endpoint, cfg.count, received, min_rtt, preserved_error);

    if result.status == "up" {
        debug!(
            monitor = %cfg.name, host = %cfg.host,
            received, sent = cfg.count, best_rtt_ms = ?min_rtt,
            "ICMP probe up"
        );
    } else {
        warn!(
            monitor = %cfg.name, host = %cfg.host,
            received, sent = cfg.count, reason = ?result.failure_reason,
            "ICMP probe down"
        );
    }

    result
}

/// Pure decision/tally logic for an ICMP cycle, kept network-free for testing.
///
/// - `received >= 1` → `up` with the best RTT; a partial loss is noted in the
///   reason while the status stays `up`.
/// - `received == 0` → `down` with the preserved non-timeout error if every echo
///   errored that way, otherwise `no_reply` with the replies/sent counts.
fn decide_icmp_result(
    name: &str,
    endpoint: &str,
    count: u32,
    received: u32,
    min_rtt: Option<u64>,
    preserved_error: Option<String>,
) -> ProbeResult {
    if received >= 1 {
        let mut result = ProbeResult::up(name, "icmp", endpoint, min_rtt.unwrap_or(0));
        if received < count {
            result.failure_reason = Some(format!("partial_loss ({received}/{count} replies)"));
        }
        result
    } else {
        let reason = preserved_error
            .unwrap_or_else(|| format!("no_reply ({received}/{count} replies)"));
        ProbeResult::down(name, "icmp", endpoint, &reason)
    }
}

fn rand_id() -> u16 {
    use std::time::{SystemTime, UNIX_EPOCH};
    (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0) & 0xFFFF) as u16
}

#[cfg(test)]
mod tests {
    use super::decide_icmp_result;

    #[test]
    fn all_replied_is_up_with_min_rtt_and_no_reason() {
        let r = decide_icmp_result("m", "1.1.1.1", 3, 3, Some(7), None);
        assert_eq!(r.status, "up");
        assert_eq!(r.response_time_ms, Some(7));
        assert_eq!(r.failure_reason, None);
    }

    #[test]
    fn partial_loss_is_up_with_partial_loss_reason() {
        let r = decide_icmp_result("m", "1.1.1.1", 3, 1, Some(12), None);
        assert_eq!(r.status, "up");
        assert_eq!(r.response_time_ms, Some(12));
        assert_eq!(r.failure_reason.as_deref(), Some("partial_loss (1/3 replies)"));
    }

    #[test]
    fn all_lost_is_down_with_no_reply_counts() {
        let r = decide_icmp_result("m", "1.1.1.1", 3, 0, None, None);
        assert_eq!(r.status, "down");
        assert_eq!(r.response_time_ms, None);
        assert_eq!(r.failure_reason.as_deref(), Some("no_reply (0/3 replies)"));
    }

    #[test]
    fn all_non_timeout_errors_preserve_error_reason() {
        let r = decide_icmp_result("m", "1.1.1.1", 2, 0, None, Some("error: boom".to_string()));
        assert_eq!(r.status, "down");
        assert_eq!(r.failure_reason.as_deref(), Some("error: boom"));
    }

    #[test]
    fn single_packet_up_on_reply() {
        let r = decide_icmp_result("m", "1.1.1.1", 1, 1, Some(4), None);
        assert_eq!(r.status, "up");
        assert_eq!(r.failure_reason, None);
    }

    #[test]
    fn single_packet_down_on_no_reply() {
        let r = decide_icmp_result("m", "1.1.1.1", 1, 0, None, None);
        assert_eq!(r.status, "down");
        assert_eq!(r.failure_reason.as_deref(), Some("no_reply (0/1 replies)"));
    }
}

use std::net::IpAddr;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::OnceLock;
use std::time::Duration;
use surge_ping::{Client, Config, PingIdentifier, PingSequence, SurgeError, ICMP};
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

/// One shared surge-ping client per address family, created on first use for
/// the process lifetime. Each `Client` owns a raw socket plus a background
/// receive task, so building one per echo (the old behavior) churned sockets
/// constantly. A creation failure (missing privilege — stable for the process
/// lifetime) is cached as `None`.
fn ping_client(ip: IpAddr) -> Option<&'static Client> {
    static V4: OnceLock<Option<Client>> = OnceLock::new();
    static V6: OnceLock<Option<Client>> = OnceLock::new();
    match ip {
        IpAddr::V4(_) => V4.get_or_init(|| Client::new(&Config::default()).ok()),
        IpAddr::V6(_) => V6.get_or_init(|| Client::new(&Config::builder().kind(ICMP::V6).build()).ok()),
    }
    .as_ref()
}

/// Unique echo identifier per ping, so concurrent pings multiplexed over the
/// shared clients can't collide on (identifier, sequence).
fn next_ident() -> u16 {
    static IDENT: AtomicU16 = AtomicU16::new(1);
    IDENT.fetch_add(1, Ordering::Relaxed)
}

/// Resolve `host` to an IP address: direct parse first, then DNS.
pub async fn resolve_host(host: &str) -> Result<IpAddr, PingError> {
    if let Ok(ip) = host.parse() {
        return Ok(ip);
    }
    match tokio::net::lookup_host(format!("{host}:0")).await {
        Ok(mut addrs) => addrs.next().map(|a| a.ip()).ok_or(PingError::Dns),
        Err(_) => Err(PingError::Dns),
    }
}

/// Send a single ICMP echo request to `host`, returning the round-trip time in
/// milliseconds. Shared by the ICMP and border probes.
pub async fn ping_once(host: &str, timeout: Duration) -> Result<u64, PingError> {
    ping_ip(resolve_host(host).await?, timeout).await
}

/// Send a single ICMP echo to an already-resolved address and await one reply
/// bounded by `timeout`. Never returns `PingError::Dns`.
pub async fn ping_ip(ip: IpAddr, timeout: Duration) -> Result<u64, PingError> {
    let client = ping_client(ip).ok_or(PingError::Privilege)?;
    let mut pinger = client.pinger(ip, PingIdentifier(next_ident())).await;
    pinger.timeout(timeout);

    match pinger.ping(PingSequence(0), &[]).await {
        Ok((_, rtt)) => Ok(rtt.as_millis() as u64),
        Err(SurgeError::Timeout { .. }) => Err(PingError::NoReply),
        Err(e) => Err(PingError::Other(format!("error: {e}"))),
    }
}

pub async fn run(cfg: &IcmpMonitorConfig) -> ProbeResult {
    let endpoint = cfg.host.clone();
    let timeout = Duration::from_millis(cfg.timeout_ms);

    // Resolve once per cycle (not per echo); a DNS failure is stable for the cycle.
    let ip = match resolve_host(&cfg.host).await {
        Ok(ip) => ip,
        Err(_) => return ProbeResult::down(&cfg.name, "icmp", &endpoint, "dns_error"),
    };

    // Send `count` echoes sequentially. Tally replies, track the best (lowest)
    // RTT, and remember the last non-timeout error. A privilege failure is
    // stable, so short-circuit on the first.
    let mut received: u32 = 0;
    let mut min_rtt: Option<u64> = None;
    let mut non_timeout_errors: u32 = 0;
    let mut last_non_timeout_error: Option<String> = None;

    for _ in 0..cfg.count {
        match ping_ip(ip, timeout).await {
            Ok(rtt) => {
                received += 1;
                min_rtt = Some(min_rtt.map_or(rtt, |m| m.min(rtt)));
            }
            Err(PingError::Dns) => unreachable!("ping_ip never returns Dns"),
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

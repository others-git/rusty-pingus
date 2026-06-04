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

pub async fn run(cfg: &IcmpMonitorConfig) -> ProbeResult {
    let endpoint = cfg.host.clone();

    let ip: IpAddr = match cfg.host.parse() {
        Ok(ip) => ip,
        Err(_) => match tokio::net::lookup_host(format!("{}:0", cfg.host)).await {
            Ok(mut addrs) => match addrs.next() {
                Some(addr) => addr.ip(),
                None => return ProbeResult::down(&cfg.name, "icmp", &endpoint, "dns_error"),
            },
            Err(_) => return ProbeResult::down(&cfg.name, "icmp", &endpoint, "dns_error"),
        },
    };

    let icmp_type = match ip {
        IpAddr::V4(_) => ICMP::V4,
        IpAddr::V6(_) => ICMP::V6,
    };

    let client = match Client::new(&Config::builder().kind(icmp_type).build()) {
        Ok(c) => c,
        Err(e) => {
            warn!(monitor = %cfg.name, error = %e, "ICMP socket error — check CAP_NET_RAW privileges");
            return ProbeResult::down(&cfg.name, "icmp", &endpoint, "privilege_error");
        }
    };

    let mut pinger = client.pinger(ip, PingIdentifier(rand_id())).await;

    pinger.timeout(Duration::from_millis(cfg.timeout_ms));

    let start = Instant::now();
    match pinger.ping(PingSequence(0), &[]).await {
        Ok(_) => {
            let elapsed = start.elapsed().as_millis() as u64;
            debug!(monitor = %cfg.name, host = %ip, elapsed_ms = elapsed, "ICMP probe success");
            ProbeResult::up(&cfg.name, "icmp", &endpoint, elapsed)
        }
        Err(e) => {
            let reason = if e.to_string().contains("timed out") || e.to_string().contains("timeout") {
                "no_reply".to_string()
            } else {
                format!("error: {e}")
            };
            ProbeResult::down(&cfg.name, "icmp", &endpoint, &reason)
        }
    }
}

fn rand_id() -> u16 {
    use std::time::{SystemTime, UNIX_EPOCH};
    (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0) & 0xFFFF) as u16
}

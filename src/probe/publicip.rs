use std::net::IpAddr;
use std::time::{Duration, Instant};
use reqwest::Client;
use sqlx::SqlitePool;
use tracing::debug;

use crate::db;
use crate::monitors::{PublicIpMonitorConfig, DEFAULT_PUBLICIP_URL, FALLBACK_PUBLICIP_URL};
use super::ProbeResult;

/// Query an external IP-echo service and record the host's public IP. Up when a
/// service responds with a parseable address (stored in `detail`, with a change
/// note when it differs from the last recorded value); down otherwise.
pub async fn run(cfg: &PublicIpMonitorConfig, pool: &SqlitePool, monitor_id: i64) -> ProbeResult {
    let endpoint = cfg
        .url
        .clone()
        .unwrap_or_else(|| DEFAULT_PUBLICIP_URL.to_string());

    let client = match Client::builder()
        .timeout(Duration::from_millis(cfg.timeout_ms))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return ProbeResult::down(&cfg.name, "publicip", &endpoint, &format!("client_build_error: {e}"))
        }
    };

    // A configured URL is used as-is; otherwise try the default then the fallback.
    let candidates: Vec<String> = match &cfg.url {
        Some(u) => vec![u.clone()],
        None => vec![DEFAULT_PUBLICIP_URL.to_string(), FALLBACK_PUBLICIP_URL.to_string()],
    };

    let mut last_reason = "no_ip_service".to_string();
    for url in &candidates {
        let start = Instant::now();
        match client.get(url).send().await {
            Ok(resp) => {
                let status = resp.status();
                if !status.is_success() {
                    last_reason = format!("unexpected_status:{}", status.as_u16());
                    continue;
                }
                match resp.text().await {
                    Ok(body) => match parse_ip_body(&body) {
                        Some(ip) => {
                            let elapsed = start.elapsed().as_millis() as u64;
                            let ip_str = ip.to_string();
                            let prev = prior_ip(pool, monitor_id).await;
                            let detail = detail_for_ip(&ip_str, prev.as_deref());
                            debug!(monitor = %cfg.name, ip = %ip_str, service = %url, "Public-IP probe up");
                            return ProbeResult::up(&cfg.name, "publicip", &endpoint, elapsed)
                                .with_detail(detail);
                        }
                        None => last_reason = "unparseable_ip".to_string(),
                    },
                    Err(e) => last_reason = format!("read_error: {e}"),
                }
            }
            Err(e) => {
                last_reason = if e.is_timeout() {
                    "timeout".to_string()
                } else {
                    format!("request_error: {e}")
                };
            }
        }
    }

    ProbeResult::down(&cfg.name, "publicip", &endpoint, &last_reason)
}

/// Parse and validate an IP-echo response body (trimmed) as an `IpAddr`.
fn parse_ip_body(body: &str) -> Option<IpAddr> {
    body.trim().parse::<IpAddr>().ok()
}

/// The detail string for a fresh reading: just the IP, or a change note carrying
/// the previous value when the address differs.
fn detail_for_ip(new_ip: &str, prev_ip: Option<&str>) -> String {
    match prev_ip {
        Some(prev) if prev != new_ip => format!("{new_ip} (changed from {prev})"),
        _ => new_ip.to_string(),
    }
}

/// The most recently recorded IP for this monitor: the leading token of the prior
/// probe's detail (which may also carry a "(changed from …)" suffix).
async fn prior_ip(pool: &SqlitePool, monitor_id: i64) -> Option<String> {
    let latest = db::get_latest_status(pool, monitor_id).await.ok().flatten()?;
    let detail = latest.detail?;
    detail.split_whitespace().next().map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_ipv4_with_trailing_newline() {
        assert_eq!(parse_ip_body("203.0.113.7\n"), Some("203.0.113.7".parse().unwrap()));
    }

    #[test]
    fn parses_valid_ipv6() {
        assert_eq!(parse_ip_body("  2001:db8::1  "), Some("2001:db8::1".parse().unwrap()));
    }

    #[test]
    fn rejects_non_ip_body() {
        assert_eq!(parse_ip_body("<html>error</html>"), None);
        assert_eq!(parse_ip_body(""), None);
    }

    #[test]
    fn detail_without_prior_is_just_the_ip() {
        assert_eq!(detail_for_ip("203.0.113.7", None), "203.0.113.7");
    }

    #[test]
    fn unchanged_ip_is_not_flagged() {
        assert_eq!(detail_for_ip("203.0.113.7", Some("203.0.113.7")), "203.0.113.7");
    }

    #[test]
    fn changed_ip_notes_previous_value() {
        assert_eq!(
            detail_for_ip("203.0.113.7", Some("198.51.100.4")),
            "203.0.113.7 (changed from 198.51.100.4)"
        );
    }
}

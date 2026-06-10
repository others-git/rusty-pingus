use std::time::{Duration, Instant};
use reqwest::Method;
use tracing::debug;

use crate::monitors::HttpMonitorConfig;
use super::ProbeResult;

pub async fn run(cfg: &HttpMonitorConfig) -> ProbeResult {
    let client = match super::shared_http_client() {
        Some(c) => c,
        None => return ProbeResult::down(&cfg.name, "http", &cfg.url, "client_build_error"),
    };

    let method = Method::from_bytes(cfg.method.as_bytes())
        .unwrap_or(Method::GET);

    let mut req = client
        .request(method, &cfg.url)
        .timeout(Duration::from_millis(cfg.timeout_ms));
    for (k, v) in &cfg.headers {
        req = req.header(k, v);
    }
    if let Some(body) = &cfg.body {
        req = req.body(body.clone());
    }

    let start = Instant::now();
    match req.send().await {
        Ok(resp) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let status = resp.status().as_u16();
            let expected = cfg.expected_status.unwrap_or(0);
            debug!(monitor = %cfg.name, status, elapsed_ms = elapsed, "HTTP probe");
            let failed = if cfg.expected_status.is_some() {
                status != expected
            } else {
                !resp.status().is_success()
            };
            if failed {
                ProbeResult::down(&cfg.name, "http", &cfg.url, &format!("unexpected_status:{status}"))
            } else {
                ProbeResult::up(&cfg.name, "http", &cfg.url, elapsed)
            }
        }
        Err(e) => {
            let reason = if e.is_timeout() {
                "timeout".to_string()
            } else if e.is_connect() {
                "connection_refused".to_string()
            } else if e.is_request() {
                format!("request_error: {e}")
            } else {
                let msg = e.to_string();
                if msg.contains("tls") || msg.contains("TLS") || msg.contains("certificate") {
                    "tls_error".to_string()
                } else if msg.contains("dns") || msg.contains("resolve") {
                    "dns_error".to_string()
                } else {
                    format!("error: {e}")
                }
            };
            ProbeResult::down(&cfg.name, "http", &cfg.url, &reason)
        }
    }
}

use std::time::{Duration, Instant};
use reqwest::{Client, Method};
use tracing::debug;

use crate::config::HttpMonitorConfig;
use super::ProbeResult;

pub async fn run(cfg: &HttpMonitorConfig) -> ProbeResult {
    let client = match Client::builder()
        .timeout(Duration::from_secs(cfg.timeout_secs))
        .build()
    {
        Ok(c) => c,
        Err(e) => return ProbeResult::down(&cfg.name, "http", &cfg.url, &format!("client_build_error: {e}")),
    };

    let method = Method::from_bytes(cfg.method.as_bytes())
        .unwrap_or(Method::GET);

    let mut req = client.request(method, &cfg.url);
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
            if expected != 0 && status != expected {
                ProbeResult::down(&cfg.name, "http", &cfg.url, &format!("unexpected_status:{status}"))
            } else if !cfg.expected_status.is_some() && !resp.status().is_success() {
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

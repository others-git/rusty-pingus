use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tracing::debug;

use crate::monitors::TcpMonitorConfig;
use super::ProbeResult;

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(host: &str, port: u16, timeout_ms: u64) -> TcpMonitorConfig {
        TcpMonitorConfig { name: "test".into(), host: host.into(), port, interval_ms: 60_000, timeout_ms, enabled: true }
    }

    #[tokio::test]
    async fn tcp_loopback_refused() {
        // Port 1 on loopback should be refused instantly
        let result = run(&cfg("127.0.0.1", 1, 3000)).await;
        assert_eq!(result.status, "down");
    }
}

pub async fn run(cfg: &TcpMonitorConfig) -> ProbeResult {
    let addr = format!("{}:{}", cfg.host, cfg.port);
    let timeout = Duration::from_millis(cfg.timeout_ms);
    let start = Instant::now();

    match tokio::time::timeout(timeout, TcpStream::connect(&addr)).await {
        Ok(Ok(_stream)) => {
            let elapsed = start.elapsed().as_millis() as u64;
            debug!(monitor = %cfg.name, addr = %addr, elapsed_ms = elapsed, "TCP probe success");
            ProbeResult::up(&cfg.name, "tcp", &addr, elapsed)
        }
        Ok(Err(e)) => {
            let reason = if e.kind() == std::io::ErrorKind::ConnectionRefused {
                "connection_refused".to_string()
            } else {
                let msg = e.to_string();
                if msg.contains("resolve") || msg.contains("dns") || msg.contains("Name or service not known") {
                    "dns_error".to_string()
                } else {
                    format!("error: {e}")
                }
            };
            ProbeResult::down(&cfg.name, "tcp", &addr, &reason)
        }
        Err(_) => ProbeResult::down(&cfg.name, "tcp", &addr, "timeout"),
    }
}

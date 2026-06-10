pub mod border;
pub mod http;
pub mod icmp;
pub mod publicip;
pub mod tcp;
pub mod traceroute;

use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::OnceLock;

/// Shared HTTP client for the http/publicip probes. Building a `reqwest::Client`
/// constructs a fresh TLS config and connection pool — far too expensive to do
/// per probe. Connection reuse is disabled (`pool_max_idle_per_host(0)`) so each
/// probe still measures a full DNS + TCP + TLS round, exactly as before; the
/// timeout is per-request since each monitor configures its own. A build failure
/// (TLS init) is cached as `None` — every probe would have failed the same way.
pub(crate) fn shared_http_client() -> Option<&'static reqwest::Client> {
    static CLIENT: OnceLock<Option<reqwest::Client>> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .pool_max_idle_per_host(0)
                .build()
                .ok()
        })
        .as_ref()
}

#[derive(Debug, Clone, Serialize)]
pub struct ProbeResult {
    pub monitor_name: String,
    pub protocol: String,
    pub endpoint: String,
    pub status: String,
    pub response_time_ms: Option<u64>,
    pub failure_reason: Option<String>,
    /// Optional, human-readable type-specific context (e.g. a public-IP monitor's
    /// observed address, or a border monitor's fault localization). Null for types
    /// that record no extra context.
    pub detail: Option<String>,
    pub checked_at: DateTime<Utc>,
}

impl ProbeResult {
    pub fn up(monitor_name: &str, protocol: &str, endpoint: &str, response_time_ms: u64) -> Self {
        Self {
            monitor_name: monitor_name.to_string(),
            protocol: protocol.to_string(),
            endpoint: endpoint.to_string(),
            status: "up".to_string(),
            response_time_ms: Some(response_time_ms),
            failure_reason: None,
            detail: None,
            checked_at: Utc::now(),
        }
    }

    pub fn down(monitor_name: &str, protocol: &str, endpoint: &str, reason: &str) -> Self {
        Self {
            monitor_name: monitor_name.to_string(),
            protocol: protocol.to_string(),
            endpoint: endpoint.to_string(),
            status: "down".to_string(),
            response_time_ms: None,
            failure_reason: Some(reason.to_string()),
            detail: None,
            checked_at: Utc::now(),
        }
    }

    /// Attach type-specific detail context, consuming and returning self so it
    /// chains off the `up`/`down` constructors.
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

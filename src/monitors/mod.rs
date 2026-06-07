use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{watch, RwLock};

use crate::config::Defaults;

pub const DEFAULT_MONITORS_CONFIG: &str = r#"# rusty-pingus monitors configuration
# Add monitors here, or use the web UI at http://localhost:3000
# Changes made via the UI are saved here automatically.

# Timing fields are in milliseconds: interval_ms and timeout_ms.
# Any monitor may set `enabled = false` to pause it (kept in config + history,
# but not probed). Omit the key for the default (enabled).
# Any monitor may set `retention_hours` to control how long its data is kept
# (default: the global retention_days × 24, e.g. 2160 h for the 90-day default).

# HTTP monitor example:
# [[monitors]]
# protocol = "http"
# name = "my-site"
# url = "https://example.com"
# interval_ms = 60000
# timeout_ms = 10000
# expected_status = 200  # optional; any 2xx accepted if omitted
# retention_hours = 2160 # optional; default is the global 90-day setting

# TCP monitor example:
# [[monitors]]
# protocol = "tcp"
# name = "my-server"
# host = "example.com"
# port = 443
# interval_ms = 30000

# ICMP monitor example (requires elevated privileges):
# [[monitors]]
# protocol = "icmp"
# name = "gateway"
# host = "1.1.1.1"
# interval_ms = 30000
# count = 3  # echo requests per cycle; up if any reply (default 3, min 1)

# Public-IP monitor example (tracks your external IP, flags changes):
# [[monitors]]
# protocol = "publicip"
# name = "my-public-ip"
# interval_ms = 300000
# url = "https://checkip.amazonaws.com"  # optional; default has a built-in fallback

# Border monitor example (localizes LAN vs ISP faults; requires ICMP privileges):
# [[monitors]]
# protocol = "border"
# name = "home-border"
# interval_ms = 30000
# gateway = "192.168.1.1"  # optional; auto-detected when omitted
# upstream = "1.1.1.1"      # upstream reference (default 1.1.1.1)

# Traceroute monitor example (per-hop path latency; requires ICMP privileges):
# [[monitors]]
# protocol = "traceroute"
# name = "path-to-cloudflare"
# host = "1.1.1.1"
# interval_ms = 5000        # minimum 500 ms is enforced for this type
# timeout_ms = 1000         # per-hop reply wait
# max_hops = 30             # optional (default 30)
# queries_per_hop = 3       # optional (default 3)
# retention_hours = 24      # optional (default 24 h); bounds stored per-hop data
"#;

// ── Monitor config types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "protocol", rename_all = "lowercase")]
pub enum MonitorConfig {
    Http(HttpMonitorConfig),
    Tcp(TcpMonitorConfig),
    Icmp(IcmpMonitorConfig),
    #[serde(rename = "publicip")]
    PublicIp(PublicIpMonitorConfig),
    Border(BorderMonitorConfig),
    Traceroute(TracerouteMonitorConfig),
}

/// Default service for the public-IP monitor; `icanhazip.com` is the fallback.
pub const DEFAULT_PUBLICIP_URL: &str = "https://checkip.amazonaws.com";
pub const FALLBACK_PUBLICIP_URL: &str = "https://icanhazip.com";
/// Default upstream reference for the border monitor.
pub const DEFAULT_BORDER_UPSTREAM: &str = "1.1.1.1";
/// Minimum scheduling interval for a traceroute monitor: a run is expensive and
/// high-volume, so the interval is clamped to this floor rather than rejected.
pub const TRACEROUTE_MIN_INTERVAL_MS: u64 = 500;

impl MonitorConfig {
    pub fn name(&self) -> &str {
        match self {
            Self::Http(c) => &c.name,
            Self::Tcp(c) => &c.name,
            Self::Icmp(c) => &c.name,
            Self::PublicIp(c) => &c.name,
            Self::Border(c) => &c.name,
            Self::Traceroute(c) => &c.name,
        }
    }

    pub fn interval_ms(&self) -> u64 {
        match self {
            Self::Http(c) => c.interval_ms,
            Self::Tcp(c) => c.interval_ms,
            Self::Icmp(c) => c.interval_ms,
            Self::PublicIp(c) => c.interval_ms,
            Self::Border(c) => c.interval_ms,
            Self::Traceroute(c) => c.interval_ms,
        }
    }

    pub fn protocol(&self) -> &'static str {
        match self {
            Self::Http(_) => "http",
            Self::Tcp(_) => "tcp",
            Self::Icmp(_) => "icmp",
            Self::PublicIp(_) => "publicip",
            Self::Border(_) => "border",
            Self::Traceroute(_) => "traceroute",
        }
    }

    /// The display endpoint, matching what the probe records in `probe_results`.
    pub fn endpoint(&self) -> String {
        match self {
            Self::Http(c) => c.url.clone(),
            Self::Tcp(c) => format!("{}:{}", c.host, c.port),
            Self::Icmp(c) => c.host.clone(),
            Self::PublicIp(c) => c.url.clone().unwrap_or_else(|| DEFAULT_PUBLICIP_URL.to_string()),
            Self::Border(c) => format!(
                "{} → {}",
                c.gateway.clone().unwrap_or_else(|| "auto".to_string()),
                c.upstream
            ),
            Self::Traceroute(c) => c.host.clone(),
        }
    }

    /// Whether this monitor should be probed. Disabled monitors are retained in
    /// config (and keep their history) but the scheduler runs no task for them.
    pub fn enabled(&self) -> bool {
        match self {
            Self::Http(c) => c.enabled,
            Self::Tcp(c) => c.enabled,
            Self::Icmp(c) => c.enabled,
            Self::PublicIp(c) => c.enabled,
            Self::Border(c) => c.enabled,
            Self::Traceroute(c) => c.enabled,
        }
    }

    /// How many hours to retain this monitor's stored data, falling back to
    /// `global_retention_days × 24` when the monitor has no explicit setting.
    pub fn retention_hours(&self, global_retention_days: u64) -> u64 {
        let raw = match self {
            Self::Http(c) => c.retention_hours,
            Self::Tcp(c) => c.retention_hours,
            Self::Icmp(c) => c.retention_hours,
            Self::PublicIp(c) => c.retention_hours,
            Self::Border(c) => c.retention_hours,
            Self::Traceroute(c) => c.retention_hours,
        };
        raw.filter(|&h| h > 0).unwrap_or(global_retention_days.saturating_mul(24))
    }

    /// Set the enabled flag on this monitor (any variant).
    fn set_enabled(&mut self, enabled: bool) {
        match self {
            Self::Http(c) => c.enabled = enabled,
            Self::Tcp(c) => c.enabled = enabled,
            Self::Icmp(c) => c.enabled = enabled,
            Self::PublicIp(c) => c.enabled = enabled,
            Self::Border(c) => c.enabled = enabled,
            Self::Traceroute(c) => c.enabled = enabled,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "RawHttpMonitorConfig")]
pub struct HttpMonitorConfig {
    pub name: String,
    pub url: String,
    pub interval_ms: u64,
    pub timeout_ms: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_status: Option<u16>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_hours: Option<u64>,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "RawTcpMonitorConfig")]
pub struct TcpMonitorConfig {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub interval_ms: u64,
    pub timeout_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_hours: Option<u64>,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "RawIcmpMonitorConfig")]
pub struct IcmpMonitorConfig {
    pub name: String,
    pub host: String,
    pub interval_ms: u64,
    pub timeout_ms: u64,
    /// Number of ICMP echo requests sent per probe cycle. The monitor is up if
    /// at least one reply is received; defaults to 3, clamped to a minimum of 1.
    pub count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_hours: Option<u64>,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "RawPublicIpMonitorConfig")]
pub struct PublicIpMonitorConfig {
    pub name: String,
    /// IP-echo service to query. Defaults to [`DEFAULT_PUBLICIP_URL`] with
    /// [`FALLBACK_PUBLICIP_URL`] as a backup when unset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub interval_ms: u64,
    pub timeout_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_hours: Option<u64>,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "RawBorderMonitorConfig")]
pub struct BorderMonitorConfig {
    pub name: String,
    /// Local gateway to probe. Auto-detected when unset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway: Option<String>,
    /// Upstream reference, defaulting to [`DEFAULT_BORDER_UPSTREAM`].
    pub upstream: String,
    pub interval_ms: u64,
    pub timeout_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_hours: Option<u64>,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "RawTracerouteMonitorConfig")]
pub struct TracerouteMonitorConfig {
    pub name: String,
    /// Target host (IP or hostname) to trace the route to.
    pub host: String,
    pub interval_ms: u64,
    pub timeout_ms: u64,
    /// Maximum number of hops (TTL) to probe before giving up. Default 30.
    pub max_hops: u32,
    /// ICMP echoes sent per hop per run; the per-run min/avg/max are computed from
    /// the responders. Default 3, clamped to a minimum of 1.
    pub queries_per_hop: u32,
    /// How long to retain this monitor's traceroute runs before pruning, in hours.
    /// When unset, falls back to the global default retention. Legacy `retention_ms`
    /// in config files is accepted and converted to hours.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_hours: Option<u64>,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub enabled: bool,
}

pub fn default_interval_ms() -> u64 { 60_000 }
pub fn default_timeout_ms() -> u64 { 10_000 }
/// Monitors default to enabled; the key is omitted from the config when true.
fn default_true() -> bool { true }
fn is_true(b: &bool) -> bool { *b }
fn default_traceroute_max_hops() -> u32 { 30 }
fn default_traceroute_queries() -> u32 { 3 }
fn default_icmp_count() -> u32 { 3 }
fn default_border_upstream() -> String { DEFAULT_BORDER_UPSTREAM.to_string() }
fn default_http_method() -> String { "GET".to_string() }

// ── Backward-compatible deserialization ───────────────────────────────────────
// Accepts both the new `*_ms` keys and the legacy `*_secs` keys. A legacy second
// value is converted to milliseconds (×1000). The `*_ms` form wins when both are
// present.

fn resolve_ms(ms: Option<u64>, secs: Option<u64>, default: u64) -> u64 {
    ms.or_else(|| secs.map(|s| s.saturating_mul(1000))).unwrap_or(default)
}

/// Resolve the standard interval/timeout pair (with legacy `*_secs` fallback)
/// shared by every monitor type's deserialization.
fn resolve_timing(
    interval_ms: Option<u64>, interval_secs: Option<u64>,
    timeout_ms: Option<u64>, timeout_secs: Option<u64>,
) -> (u64, u64) {
    (
        resolve_ms(interval_ms, interval_secs, default_interval_ms()),
        resolve_ms(timeout_ms, timeout_secs, default_timeout_ms()),
    )
}

/// Apply global `[defaults]` to a monitor's interval/timeout: a per-monitor value
/// still at the built-in default is overridden by the configured default.
fn apply_timing_defaults(interval_ms: &mut u64, timeout_ms: &mut u64, defaults: &Defaults) {
    if let Some(t) = defaults.timeout_ms { if *timeout_ms == default_timeout_ms() { *timeout_ms = t; } }
    if let Some(i) = defaults.interval_ms { if *interval_ms == default_interval_ms() { *interval_ms = i; } }
}

#[derive(Deserialize)]
struct RawHttpMonitorConfig {
    name: String,
    url: String,
    #[serde(default)] interval_ms: Option<u64>,
    #[serde(default)] interval_secs: Option<u64>,
    #[serde(default)] timeout_ms: Option<u64>,
    #[serde(default)] timeout_secs: Option<u64>,
    #[serde(default = "default_http_method")] method: String,
    #[serde(default)] expected_status: Option<u16>,
    #[serde(default)] headers: HashMap<String, String>,
    #[serde(default)] body: Option<String>,
    #[serde(default)] retention_hours: Option<u64>,
    #[serde(default = "default_true")] enabled: bool,
}

impl From<RawHttpMonitorConfig> for HttpMonitorConfig {
    fn from(r: RawHttpMonitorConfig) -> Self {
        let (interval_ms, timeout_ms) = resolve_timing(r.interval_ms, r.interval_secs, r.timeout_ms, r.timeout_secs);
        Self {
            name: r.name,
            url: r.url,
            interval_ms,
            timeout_ms,
            method: r.method,
            expected_status: r.expected_status,
            headers: r.headers,
            body: r.body,
            retention_hours: r.retention_hours.filter(|&h| h > 0),
            enabled: r.enabled,
        }
    }
}

#[derive(Deserialize)]
struct RawTcpMonitorConfig {
    name: String,
    host: String,
    port: u16,
    #[serde(default)] interval_ms: Option<u64>,
    #[serde(default)] interval_secs: Option<u64>,
    #[serde(default)] timeout_ms: Option<u64>,
    #[serde(default)] timeout_secs: Option<u64>,
    #[serde(default)] retention_hours: Option<u64>,
    #[serde(default = "default_true")] enabled: bool,
}

impl From<RawTcpMonitorConfig> for TcpMonitorConfig {
    fn from(r: RawTcpMonitorConfig) -> Self {
        let (interval_ms, timeout_ms) = resolve_timing(r.interval_ms, r.interval_secs, r.timeout_ms, r.timeout_secs);
        Self { name: r.name, host: r.host, port: r.port, interval_ms, timeout_ms,
               retention_hours: r.retention_hours.filter(|&h| h > 0), enabled: r.enabled }
    }
}

#[derive(Deserialize)]
struct RawIcmpMonitorConfig {
    name: String,
    host: String,
    #[serde(default)] interval_ms: Option<u64>,
    #[serde(default)] interval_secs: Option<u64>,
    #[serde(default)] timeout_ms: Option<u64>,
    #[serde(default)] timeout_secs: Option<u64>,
    #[serde(default)] count: Option<u32>,
    #[serde(default)] retention_hours: Option<u64>,
    #[serde(default = "default_true")] enabled: bool,
}

impl From<RawIcmpMonitorConfig> for IcmpMonitorConfig {
    fn from(r: RawIcmpMonitorConfig) -> Self {
        let (interval_ms, timeout_ms) = resolve_timing(r.interval_ms, r.interval_secs, r.timeout_ms, r.timeout_secs);
        Self {
            enabled: r.enabled,
            name: r.name,
            host: r.host,
            interval_ms,
            timeout_ms,
            count: r.count.unwrap_or_else(default_icmp_count).max(1),
            retention_hours: r.retention_hours.filter(|&h| h > 0),
        }
    }
}

#[derive(Deserialize)]
struct RawPublicIpMonitorConfig {
    name: String,
    #[serde(default)] url: Option<String>,
    #[serde(default)] interval_ms: Option<u64>,
    #[serde(default)] interval_secs: Option<u64>,
    #[serde(default)] timeout_ms: Option<u64>,
    #[serde(default)] timeout_secs: Option<u64>,
    #[serde(default)] retention_hours: Option<u64>,
    #[serde(default = "default_true")] enabled: bool,
}

impl From<RawPublicIpMonitorConfig> for PublicIpMonitorConfig {
    fn from(r: RawPublicIpMonitorConfig) -> Self {
        let (interval_ms, timeout_ms) = resolve_timing(r.interval_ms, r.interval_secs, r.timeout_ms, r.timeout_secs);
        Self {
            name: r.name,
            // Treat an empty URL as unset so the default/fallback applies.
            url: r.url.filter(|u| !u.trim().is_empty()),
            interval_ms,
            timeout_ms,
            retention_hours: r.retention_hours.filter(|&h| h > 0),
            enabled: r.enabled,
        }
    }
}

#[derive(Deserialize)]
struct RawBorderMonitorConfig {
    name: String,
    #[serde(default)] gateway: Option<String>,
    #[serde(default)] upstream: Option<String>,
    #[serde(default)] interval_ms: Option<u64>,
    #[serde(default)] interval_secs: Option<u64>,
    #[serde(default)] timeout_ms: Option<u64>,
    #[serde(default)] timeout_secs: Option<u64>,
    #[serde(default)] retention_hours: Option<u64>,
    #[serde(default = "default_true")] enabled: bool,
}

impl From<RawBorderMonitorConfig> for BorderMonitorConfig {
    fn from(r: RawBorderMonitorConfig) -> Self {
        let (interval_ms, timeout_ms) = resolve_timing(r.interval_ms, r.interval_secs, r.timeout_ms, r.timeout_secs);
        Self {
            name: r.name,
            gateway: r.gateway.filter(|g| !g.trim().is_empty()),
            upstream: r
                .upstream
                .filter(|u| !u.trim().is_empty())
                .unwrap_or_else(default_border_upstream),
            interval_ms,
            timeout_ms,
            retention_hours: r.retention_hours.filter(|&h| h > 0),
            enabled: r.enabled,
        }
    }
}

#[derive(Deserialize)]
struct RawTracerouteMonitorConfig {
    name: String,
    host: String,
    #[serde(default)] interval_ms: Option<u64>,
    #[serde(default)] interval_secs: Option<u64>,
    #[serde(default)] timeout_ms: Option<u64>,
    #[serde(default)] timeout_secs: Option<u64>,
    #[serde(default)] max_hops: Option<u32>,
    #[serde(default)] queries_per_hop: Option<u32>,
    /// Legacy field: retained for back-compat deserialization, converted to hours.
    #[serde(default)] retention_ms: Option<u64>,
    #[serde(default)] retention_secs: Option<u64>,
    /// New unified field: wins over legacy `retention_ms` when both are present.
    #[serde(default)] retention_hours: Option<u64>,
    #[serde(default = "default_true")] enabled: bool,
}

impl From<RawTracerouteMonitorConfig> for TracerouteMonitorConfig {
    fn from(r: RawTracerouteMonitorConfig) -> Self {
        let (interval_ms, timeout_ms) = resolve_timing(r.interval_ms, r.interval_secs, r.timeout_ms, r.timeout_secs);
        // Unified retention: prefer the new `retention_hours`; fall back to the legacy
        // `retention_ms`/`retention_secs` pair (converted to hours, minimum 1 h).
        let legacy_ms = r.retention_ms.or_else(|| r.retention_secs.map(|s| s.saturating_mul(1000)));
        let retention_hours = r.retention_hours.filter(|&h| h > 0)
            .or_else(|| legacy_ms.map(|ms| (ms / 3_600_000).max(1)));
        Self {
            name: r.name,
            host: r.host,
            // Enforce the 500 ms interval floor uniformly across config + UI paths.
            interval_ms: interval_ms.max(TRACEROUTE_MIN_INTERVAL_MS),
            timeout_ms,
            max_hops: r.max_hops.unwrap_or_else(default_traceroute_max_hops).clamp(1, 64),
            queries_per_hop: r.queries_per_hop.unwrap_or_else(default_traceroute_queries).max(1),
            retention_hours,
            enabled: r.enabled,
        }
    }
}

// ── File wrapper for TOML serialization ──────────────────────────────────────

#[derive(Serialize, Deserialize, Default)]
struct MonitorsFile {
    #[serde(default)]
    monitors: Vec<MonitorConfig>,
}

// ── MonitorStore ──────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct MonitorStore {
    inner: Arc<RwLock<Vec<MonitorConfig>>>,
    path: PathBuf,
    tx: watch::Sender<Vec<MonitorConfig>>,
}

impl MonitorStore {
    /// Load monitors from `path`, applying `defaults` from config.toml.
    /// Returns the store and a watch receiver for hot-reload notifications.
    pub fn load(
        path: &Path,
        defaults: &Defaults,
    ) -> Result<(Self, watch::Receiver<Vec<MonitorConfig>>)> {
        let (monitors, legacy_found) = load_from_path(path, defaults)?;

        // If the file used the legacy `*_secs` keys, the values were converted to
        // milliseconds on load; rewrite the file once in the canonical `*_ms` form.
        if legacy_found {
            match save_to_path(path, &monitors) {
                Ok(()) => tracing::warn!(
                    path = %path.display(),
                    "Converted legacy *_secs timing fields to milliseconds (×1000) and rewrote the monitors file"
                ),
                Err(e) => tracing::error!(error = %e, "Failed to rewrite monitors file after millisecond migration"),
            }
        }

        let (tx, rx) = watch::channel(monitors.clone());
        let store = Self {
            inner: Arc::new(RwLock::new(monitors)),
            path: path.to_path_buf(),
            tx,
        };
        Ok((store, rx))
    }

    pub async fn list(&self) -> Vec<MonitorConfig> {
        self.inner.read().await.clone()
    }

    pub async fn add(&self, monitor: MonitorConfig) -> Result<()> {
        let mut monitors = self.inner.write().await;
        if monitors.iter().any(|m| m.name() == monitor.name()) {
            anyhow::bail!("a monitor named '{}' already exists", monitor.name());
        }
        monitors.push(monitor);
        self.persist_and_notify(&monitors)?;
        Ok(())
    }

    /// Set a monitor's enabled flag, persisting and notifying watchers (the
    /// scheduler reacts via hot-reload). Returns `false` if no such monitor.
    pub async fn set_enabled(&self, name: &str, enabled: bool) -> Result<bool> {
        let mut monitors = self.inner.write().await;
        let Some(m) = monitors.iter_mut().find(|m| m.name() == name) else {
            return Ok(false);
        };
        if m.enabled() == enabled {
            return Ok(true); // no-op; avoid a needless rewrite/notify
        }
        m.set_enabled(enabled);
        self.persist_and_notify(&monitors)?;
        Ok(true)
    }

    /// Returns `true` if found and removed, `false` if not found.
    pub async fn remove(&self, name: &str) -> Result<bool> {
        let mut monitors = self.inner.write().await;
        let before = monitors.len();
        monitors.retain(|m| m.name() != name);
        if monitors.len() == before {
            return Ok(false);
        }
        self.persist_and_notify(&monitors)?;
        Ok(true)
    }

    fn persist_and_notify(&self, monitors: &[MonitorConfig]) -> Result<()> {
        save_to_path(&self.path, monitors)?;
        let _ = self.tx.send(monitors.to_vec());
        Ok(())
    }
}

/// Returns the loaded monitors and whether the file used legacy `*_secs` keys
/// (which were converted to milliseconds), signalling that a rewrite is due.
fn load_from_path(path: &Path, defaults: &Defaults) -> Result<(Vec<MonitorConfig>, bool)> {
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(path, DEFAULT_MONITORS_CONFIG)
            .with_context(|| format!("Could not write default monitors config to: {}", path.display()))?;
        tracing::warn!(
            path = %path.display(),
            "No monitors.toml found — generated a default. Add monitors via the web UI or edit the file."
        );
        return Ok((vec![], false));
    }

    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("Could not read monitors file: {}", path.display()))?;
    let file: MonitorsFile = toml::from_str(&contents)
        .with_context(|| format!("Invalid TOML in monitors file: {}", path.display()))?;

    let legacy_found = contents.contains("interval_secs") || contents.contains("timeout_secs");

    let mut monitors = file.monitors;
    apply_defaults(&mut monitors, defaults);
    Ok((monitors, legacy_found))
}

fn save_to_path(path: &Path, monitors: &[MonitorConfig]) -> Result<()> {
    let file = MonitorsFile { monitors: monitors.to_vec() };
    let contents = toml::to_string_pretty(&file)
        .context("Failed to serialise monitors to TOML")?;

    // Atomic write: write to temp file then rename
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, contents)
        .with_context(|| format!("Failed to write temp monitors file: {}", tmp.display()))?;
    std::fs::rename(&tmp, path)
        .with_context(|| format!("Failed to rename monitors file: {}", path.display()))?;
    Ok(())
}

pub fn apply_defaults(monitors: &mut Vec<MonitorConfig>, defaults: &Defaults) {
    for monitor in monitors {
        match monitor {
            MonitorConfig::Http(c) => apply_timing_defaults(&mut c.interval_ms, &mut c.timeout_ms, defaults),
            MonitorConfig::Tcp(c) => apply_timing_defaults(&mut c.interval_ms, &mut c.timeout_ms, defaults),
            MonitorConfig::Icmp(c) => apply_timing_defaults(&mut c.interval_ms, &mut c.timeout_ms, defaults),
            MonitorConfig::PublicIp(c) => apply_timing_defaults(&mut c.interval_ms, &mut c.timeout_ms, defaults),
            MonitorConfig::Border(c) => apply_timing_defaults(&mut c.interval_ms, &mut c.timeout_ms, defaults),
            MonitorConfig::Traceroute(c) => {
                apply_timing_defaults(&mut c.interval_ms, &mut c.timeout_ms, defaults);
                // Re-apply the floor in case a global default lowered the interval.
                c.interval_ms = c.interval_ms.max(TRACEROUTE_MIN_INTERVAL_MS);
            }
        }
    }
}

// ── Migration helper ──────────────────────────────────────────────────────────

/// If config.toml had [[monitors]] entries (from the old format), migrate them
/// to monitors.toml.  The legacy monitors are passed in already parsed.
pub fn migrate_legacy_monitors(
    legacy: Vec<MonitorConfig>,
    monitors_path: &Path,
    config_path: &Path,
) -> Result<()> {
    if legacy.is_empty() {
        return Ok(());
    }

    tracing::warn!(
        count = legacy.len(),
        monitors_path = %monitors_path.display(),
        "Migrating [[monitors]] from config.toml to monitors.toml — \
         these entries will be removed from config.toml"
    );

    // Write monitors to monitors.toml (create or overwrite)
    save_to_path(monitors_path, &legacy)?;

    // Rewrite config.toml: strip [[monitors]] blocks by re-serialising without them
    // We do a naive text-based approach: strip lines from the first [[monitors]] onwards.
    if let Ok(contents) = std::fs::read_to_string(config_path) {
        let stripped = strip_monitors_from_toml(&contents);
        std::fs::write(config_path, stripped)
            .with_context(|| format!("Failed to rewrite {}", config_path.display()))?;
    }

    Ok(())
}

/// Remove all `[[monitors]]` table entries from TOML text.
fn strip_monitors_from_toml(input: &str) -> String {
    let mut output = Vec::new();
    let mut in_monitors = false;
    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("[[monitors]]") {
            in_monitors = true;
            continue;
        }
        if in_monitors {
            // Any new section header ends the monitors block
            if trimmed.starts_with('[') && !trimmed.starts_with('#') {
                in_monitors = false;
            } else {
                continue;
            }
        }
        output.push(line);
    }
    let mut result = output.join("\n");
    if !result.ends_with('\n') { result.push('\n'); }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_icmp(toml_body: &str) -> IcmpMonitorConfig {
        let file: MonitorsFile = toml::from_str(toml_body).expect("valid TOML");
        match file.monitors.into_iter().next().expect("one monitor") {
            MonitorConfig::Icmp(c) => c,
            other => panic!("expected ICMP monitor, got {other:?}"),
        }
    }

    #[test]
    fn icmp_count_defaults_to_three_when_missing() {
        let c = parse_icmp(
            "[[monitors]]\nprotocol = \"icmp\"\nname = \"gw\"\nhost = \"1.1.1.1\"\n",
        );
        assert_eq!(c.count, 3);
    }

    #[test]
    fn icmp_count_zero_is_clamped_to_one() {
        let c = parse_icmp(
            "[[monitors]]\nprotocol = \"icmp\"\nname = \"gw\"\nhost = \"1.1.1.1\"\ncount = 0\n",
        );
        assert_eq!(c.count, 1);
    }

    #[test]
    fn icmp_count_explicit_value_is_respected() {
        let c = parse_icmp(
            "[[monitors]]\nprotocol = \"icmp\"\nname = \"gw\"\nhost = \"1.1.1.1\"\ncount = 5\n",
        );
        assert_eq!(c.count, 5);
    }

    fn parse_traceroute(toml_body: &str) -> TracerouteMonitorConfig {
        let file: MonitorsFile = toml::from_str(toml_body).expect("valid TOML");
        match file.monitors.into_iter().next().expect("one monitor") {
            MonitorConfig::Traceroute(c) => c,
            other => panic!("expected traceroute monitor, got {other:?}"),
        }
    }

    #[test]
    fn traceroute_sub_floor_interval_is_raised_to_500() {
        let c = parse_traceroute(
            "[[monitors]]\nprotocol = \"traceroute\"\nname = \"t\"\nhost = \"1.1.1.1\"\ninterval_ms = 100\n",
        );
        assert_eq!(c.interval_ms, 500);
    }

    #[test]
    fn traceroute_at_or_above_floor_interval_is_preserved() {
        let c = parse_traceroute(
            "[[monitors]]\nprotocol = \"traceroute\"\nname = \"t\"\nhost = \"1.1.1.1\"\ninterval_ms = 5000\n",
        );
        assert_eq!(c.interval_ms, 5000);
    }

    #[test]
    fn traceroute_defaults_for_optional_fields() {
        let c = parse_traceroute(
            "[[monitors]]\nprotocol = \"traceroute\"\nname = \"t\"\nhost = \"1.1.1.1\"\n",
        );
        assert_eq!(c.max_hops, 30);
        assert_eq!(c.queries_per_hop, 3);
        // No explicit retention → None (falls back to global default at runtime)
        assert_eq!(c.retention_hours, None);
    }

    #[test]
    fn traceroute_legacy_retention_ms_converts_to_hours() {
        let c = parse_traceroute(
            "[[monitors]]\nprotocol = \"traceroute\"\nname = \"t\"\nhost = \"1.1.1.1\"\nretention_ms = 86400000\n",
        );
        assert_eq!(c.retention_hours, Some(24));
    }

    #[test]
    fn retention_hours_explicit_wins_over_legacy_ms() {
        let c = parse_traceroute(
            "[[monitors]]\nprotocol = \"traceroute\"\nname = \"t\"\nhost = \"1.1.1.1\"\nretention_hours = 6\nretention_ms = 86400000\n",
        );
        assert_eq!(c.retention_hours, Some(6));
    }

    #[test]
    fn retention_hours_accessor_falls_back_to_global() {
        let c = parse_traceroute(
            "[[monitors]]\nprotocol = \"traceroute\"\nname = \"t\"\nhost = \"1.1.1.1\"\n",
        );
        let m = MonitorConfig::Traceroute(c);
        // No per-monitor retention → uses global_days * 24
        assert_eq!(m.retention_hours(90), 2160);
    }

    #[test]
    fn retention_hours_accessor_uses_per_monitor_value() {
        let c = parse_traceroute(
            "[[monitors]]\nprotocol = \"traceroute\"\nname = \"t\"\nhost = \"1.1.1.1\"\nretention_hours = 6\n",
        );
        let m = MonitorConfig::Traceroute(c);
        assert_eq!(m.retention_hours(90), 6);
    }

    #[test]
    fn enabled_defaults_to_true_when_missing() {
        let c = parse_traceroute(
            "[[monitors]]\nprotocol = \"traceroute\"\nname = \"t\"\nhost = \"1.1.1.1\"\n",
        );
        assert!(c.enabled);
    }

    #[test]
    fn enabled_false_round_trips() {
        let toml_in = "[[monitors]]\nprotocol = \"http\"\nname = \"s\"\nurl = \"https://x\"\nenabled = false\n";
        let file: MonitorsFile = toml::from_str(toml_in).expect("valid TOML");
        match &file.monitors[0] {
            MonitorConfig::Http(c) => assert!(!c.enabled),
            other => panic!("expected http, got {other:?}"),
        }
        // Serializing a disabled monitor must keep the key; an enabled one omits it.
        let out = toml::to_string(&file).unwrap();
        assert!(out.contains("enabled = false"), "disabled flag must persist: {out}");
    }

    #[test]
    fn enabled_true_is_omitted_on_serialize() {
        let file = MonitorsFile {
            monitors: vec![MonitorConfig::Http(HttpMonitorConfig {
                name: "s".into(), url: "https://x".into(), interval_ms: 1000, timeout_ms: 1000,
                method: "GET".into(), expected_status: None, headers: HashMap::new(), body: None,
                retention_hours: None,
                enabled: true,
            })],
        };
        let out = toml::to_string(&file).unwrap();
        assert!(!out.contains("enabled"), "enabled=true should be omitted: {out}");
    }

    #[test]
    fn traceroute_zero_queries_clamped_to_one() {
        let c = parse_traceroute(
            "[[monitors]]\nprotocol = \"traceroute\"\nname = \"t\"\nhost = \"1.1.1.1\"\nqueries_per_hop = 0\n",
        );
        assert_eq!(c.queries_per_hop, 1);
    }
}

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

# HTTP monitor example:
# [[monitors]]
# protocol = "http"
# name = "my-site"
# url = "https://example.com"
# interval_ms = 60000
# timeout_ms = 10000
# expected_status = 200  # optional; any 2xx accepted if omitted

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
"#;

// ── Monitor config types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "protocol", rename_all = "lowercase")]
pub enum MonitorConfig {
    Http(HttpMonitorConfig),
    Tcp(TcpMonitorConfig),
    Icmp(IcmpMonitorConfig),
}

impl MonitorConfig {
    pub fn name(&self) -> &str {
        match self {
            Self::Http(c) => &c.name,
            Self::Tcp(c) => &c.name,
            Self::Icmp(c) => &c.name,
        }
    }

    pub fn interval_ms(&self) -> u64 {
        match self {
            Self::Http(c) => c.interval_ms,
            Self::Tcp(c) => c.interval_ms,
            Self::Icmp(c) => c.interval_ms,
        }
    }

    pub fn protocol(&self) -> &'static str {
        match self {
            Self::Http(_) => "http",
            Self::Tcp(_) => "tcp",
            Self::Icmp(_) => "icmp",
        }
    }

    /// The display endpoint, matching what the probe records in `probe_results`.
    pub fn endpoint(&self) -> String {
        match self {
            Self::Http(c) => c.url.clone(),
            Self::Tcp(c) => format!("{}:{}", c.host, c.port),
            Self::Icmp(c) => c.host.clone(),
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "RawTcpMonitorConfig")]
pub struct TcpMonitorConfig {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub interval_ms: u64,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "RawIcmpMonitorConfig")]
pub struct IcmpMonitorConfig {
    pub name: String,
    pub host: String,
    pub interval_ms: u64,
    pub timeout_ms: u64,
}

pub fn default_interval_ms() -> u64 { 60_000 }
pub fn default_timeout_ms() -> u64 { 10_000 }
fn default_http_method() -> String { "GET".to_string() }

// ── Backward-compatible deserialization ───────────────────────────────────────
// Accepts both the new `*_ms` keys and the legacy `*_secs` keys. A legacy second
// value is converted to milliseconds (×1000). The `*_ms` form wins when both are
// present.

fn resolve_ms(ms: Option<u64>, secs: Option<u64>, default: u64) -> u64 {
    ms.or_else(|| secs.map(|s| s.saturating_mul(1000))).unwrap_or(default)
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
}

impl From<RawHttpMonitorConfig> for HttpMonitorConfig {
    fn from(r: RawHttpMonitorConfig) -> Self {
        Self {
            name: r.name,
            url: r.url,
            interval_ms: resolve_ms(r.interval_ms, r.interval_secs, default_interval_ms()),
            timeout_ms: resolve_ms(r.timeout_ms, r.timeout_secs, default_timeout_ms()),
            method: r.method,
            expected_status: r.expected_status,
            headers: r.headers,
            body: r.body,
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
}

impl From<RawTcpMonitorConfig> for TcpMonitorConfig {
    fn from(r: RawTcpMonitorConfig) -> Self {
        Self {
            name: r.name,
            host: r.host,
            port: r.port,
            interval_ms: resolve_ms(r.interval_ms, r.interval_secs, default_interval_ms()),
            timeout_ms: resolve_ms(r.timeout_ms, r.timeout_secs, default_timeout_ms()),
        }
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
}

impl From<RawIcmpMonitorConfig> for IcmpMonitorConfig {
    fn from(r: RawIcmpMonitorConfig) -> Self {
        Self {
            name: r.name,
            host: r.host,
            interval_ms: resolve_ms(r.interval_ms, r.interval_secs, default_interval_ms()),
            timeout_ms: resolve_ms(r.timeout_ms, r.timeout_secs, default_timeout_ms()),
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
    let timeout = defaults.timeout_ms;
    let interval = defaults.interval_ms;
    for monitor in monitors {
        match monitor {
            MonitorConfig::Http(c) => {
                if let Some(t) = timeout { if c.timeout_ms == default_timeout_ms() { c.timeout_ms = t; } }
                if let Some(i) = interval { if c.interval_ms == default_interval_ms() { c.interval_ms = i; } }
            }
            MonitorConfig::Tcp(c) => {
                if let Some(t) = timeout { if c.timeout_ms == default_timeout_ms() { c.timeout_ms = t; } }
                if let Some(i) = interval { if c.interval_ms == default_interval_ms() { c.interval_ms = i; } }
            }
            MonitorConfig::Icmp(c) => {
                if let Some(t) = timeout { if c.timeout_ms == default_timeout_ms() { c.timeout_ms = t; } }
                if let Some(i) = interval { if c.interval_ms == default_interval_ms() { c.interval_ms = i; } }
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

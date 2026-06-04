use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Default retention applied when `[defaults].retention_days` is unset, so the
/// probe_results table is pruned by age out of the box rather than growing forever.
pub const DEFAULT_RETENTION_DAYS: u64 = 90;

pub const DEFAULT_CONFIG: &str = r#"# rusty-pingus configuration
# Generated automatically — edit to update app settings and restart.
# Monitors are managed separately in monitors.toml (or via the web UI).

[defaults]
timeout_ms = 10000
interval_ms = 60000
# retention_days defaults to 90 when unset; uncomment to override (e.g. keep longer).
# retention_days = 90

[web]
bind = "0.0.0.0:3000"

[database]
path = "./data/rusty-pingus.db"

[monitors]
path = "./monitors.toml"
"#;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub web: WebConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub monitors: MonitorsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(from = "RawDefaults")]
pub struct Defaults {
    pub timeout_ms: Option<u64>,
    pub interval_ms: Option<u64>,
    pub retention_days: Option<u64>,
}

/// Accepts both `*_ms` and legacy `*_secs` keys; legacy values are converted to
/// milliseconds (×1000). The `*_ms` form wins when both are present.
#[derive(Deserialize, Default)]
struct RawDefaults {
    #[serde(default)] timeout_ms: Option<u64>,
    #[serde(default)] timeout_secs: Option<u64>,
    #[serde(default)] interval_ms: Option<u64>,
    #[serde(default)] interval_secs: Option<u64>,
    #[serde(default)] retention_days: Option<u64>,
}

impl From<RawDefaults> for Defaults {
    fn from(r: RawDefaults) -> Self {
        Self {
            timeout_ms: r.timeout_ms.or_else(|| r.timeout_secs.map(|s| s.saturating_mul(1000))),
            interval_ms: r.interval_ms.or_else(|| r.interval_secs.map(|s| s.saturating_mul(1000))),
            retention_days: r.retention_days,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    pub bind: String,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self { bind: "0.0.0.0:3000".to_string() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self { path: "./data/rusty-pingus.db".to_string() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorsConfig {
    pub path: String,
}

impl Default for MonitorsConfig {
    fn default() -> Self {
        Self { path: "./monitors.toml".to_string() }
    }
}

pub fn load(path: &Path) -> Result<Config> {
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(path, DEFAULT_CONFIG)
            .with_context(|| format!("Could not write default config to: {}", path.display()))?;
        tracing::warn!(
            path = %path.display(),
            "No config found — generated a default. Edit it to configure the app."
        );
        return Ok(Config::default());
    }

    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("Could not read config file: {}", path.display()))?;
    let config: Config = toml::from_str(&contents)
        .with_context(|| format!("Invalid TOML in config file: {}", path.display()))?;
    Ok(config)
}

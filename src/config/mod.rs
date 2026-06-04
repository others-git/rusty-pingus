use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub web: WebConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub monitors: Vec<MonitorConfig>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Defaults {
    pub timeout_secs: Option<u64>,
    pub interval_secs: Option<u64>,
    pub retention_days: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebConfig {
    pub bind: String,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self { bind: "0.0.0.0:3000".to_string() }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self { path: "./data/rusty-pingus.db".to_string() }
    }
}

#[derive(Debug, Clone, Deserialize)]
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

    pub fn interval_secs(&self) -> u64 {
        match self {
            Self::Http(c) => c.interval_secs,
            Self::Tcp(c) => c.interval_secs,
            Self::Icmp(c) => c.interval_secs,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct HttpMonitorConfig {
    pub name: String,
    pub url: String,
    #[serde(default = "default_interval")]
    pub interval_secs: u64,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default = "default_http_method")]
    pub method: String,
    pub expected_status: Option<u16>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TcpMonitorConfig {
    pub name: String,
    pub host: String,
    pub port: u16,
    #[serde(default = "default_interval")]
    pub interval_secs: u64,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IcmpMonitorConfig {
    pub name: String,
    pub host: String,
    #[serde(default = "default_interval")]
    pub interval_secs: u64,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_interval() -> u64 { 60 }
fn default_timeout() -> u64 { 10 }
fn default_http_method() -> String { "GET".to_string() }

pub fn load(path: &Path) -> Result<Config> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("Could not read config file: {}", path.display()))?;
    let mut config: Config = toml::from_str(&contents)
        .with_context(|| format!("Invalid TOML in config file: {}", path.display()))?;
    apply_defaults(&mut config);
    Ok(config)
}

fn apply_defaults(config: &mut Config) {
    let timeout = config.defaults.timeout_secs;
    let interval = config.defaults.interval_secs;
    for monitor in &mut config.monitors {
        match monitor {
            MonitorConfig::Http(c) => {
                if let Some(t) = timeout { if c.timeout_secs == default_timeout() { c.timeout_secs = t; } }
                if let Some(i) = interval { if c.interval_secs == default_interval() { c.interval_secs = i; } }
            }
            MonitorConfig::Tcp(c) => {
                if let Some(t) = timeout { if c.timeout_secs == default_timeout() { c.timeout_secs = t; } }
                if let Some(i) = interval { if c.interval_secs == default_interval() { c.interval_secs = i; } }
            }
            MonitorConfig::Icmp(c) => {
                if let Some(t) = timeout { if c.timeout_secs == default_timeout() { c.timeout_secs = t; } }
                if let Some(i) = interval { if c.interval_secs == default_interval() { c.interval_secs = i; } }
            }
        }
    }
}

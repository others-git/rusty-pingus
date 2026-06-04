#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use rusty_pingus::{api::AppState, config, db, monitors, probe, scheduler, web};
#[cfg(windows)]
use rusty_pingus::tray;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "rusty-pingus", about = "Self-hosted uptime monitor")]
pub struct Cli {
    #[arg(long, default_value = "./config.toml")]
    pub config: PathBuf,
    #[arg(long)]
    pub bind: Option<String>,
    #[arg(long)]
    pub db: Option<String>,
    /// Path to monitors config file (overrides config.toml setting)
    #[arg(long)]
    pub monitors: Option<PathBuf>,
}

fn dashboard_url(bind: &str) -> String {
    format!("http://{}", bind.replace("0.0.0.0", "localhost"))
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Console logging is initialised up front so startup work (e.g. migration)
    // is visible. On Windows release builds, file logging is set up later once
    // the database path is known; the two are mutually exclusive via cfg.
    #[cfg(not(all(windows, not(debug_assertions))))]
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
        .init();

    // Migrate legacy `[[monitors]]` entries out of config.toml *before* parsing
    // the config: the current Config schema uses `[monitors]` (a table holding the
    // monitors-file path) and rejects the old array-of-tables format, so a legacy
    // config.toml would otherwise fail to parse.
    let migration_monitors_path = cli
        .monitors
        .clone()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "./monitors.toml".to_string());
    migrate_legacy_config(&cli.config, std::path::Path::new(&migration_monitors_path));

    let mut cfg = config::load(&cli.config)?;
    if let Some(bind) = cli.bind { cfg.web.bind = bind; }
    if let Some(db_path) = cli.db { cfg.database.path = db_path; }
    if let Some(m) = cli.monitors { cfg.monitors.path = m.to_string_lossy().into_owned(); }

    if let Some(parent) = std::path::Path::new(&cfg.database.path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let is_first_launch = !std::path::Path::new(&cfg.database.path).exists();
    let url = dashboard_url(&cfg.web.bind);

    // ── Logging ───────────────────────────────────────────────────────────────
    #[cfg(all(windows, not(debug_assertions)))]
    let _log_guard = {
        let log_dir = std::path::Path::new(&cfg.database.path)
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("logs");
        std::fs::create_dir_all(&log_dir).ok();
        let file_appender = tracing_appender::rolling::daily(&log_dir, "rusty-pingus.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
            .with_writer(non_blocking)
            .with_ansi(false)
            .init();
        guard
    };

    // ── Platform entry ────────────────────────────────────────────────────────
    #[cfg(windows)]
    {
        let cancel = CancellationToken::new();
        let cancel_bg = cancel.clone();
        let cfg_bg = cfg;
        let url_bg = url.clone();
        let first_launch = is_first_launch;
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            if let Err(e) = rt.block_on(async_main(cfg_bg, first_launch, url_bg, cancel_bg)) {
                tracing::error!("Fatal: {e}");
            }
        });
        std::thread::sleep(std::time::Duration::from_millis(800));
        if is_first_launch { let _ = webbrowser::open(&url); }
        tray::run_event_loop(url, cancel);
    }

    #[cfg(not(windows))]
    {
        let cancel = CancellationToken::new();
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async_main(cfg, is_first_launch, url, cancel))?;
        Ok(())
    }
}

async fn async_main(
    cfg: config::Config,
    is_first_launch: bool,
    dashboard_url: String,
    cancel: CancellationToken,
) -> Result<()> {
    let monitors_path = std::path::Path::new(&cfg.monitors.path);

    // ── Load monitors ─────────────────────────────────────────────────────────
    let (monitor_store, monitor_rx) = monitors::MonitorStore::load(monitors_path, &cfg.defaults)?;
    let initial_monitors = monitor_store.list().await;

    let pool = db::init(&cfg.database.path).await?;

    info!("Starting rusty-pingus with {} monitors", initial_monitors.len());

    let has_icmp = initial_monitors.iter().any(|m| matches!(m, monitors::MonitorConfig::Icmp(_)));
    if has_icmp && !probe::icmp::check_privilege() {
        tracing::warn!("ICMP monitors configured but CAP_NET_RAW may be unavailable.");
    }

    let state = AppState { pool: pool.clone(), monitors: monitor_store };

    let sched_handle = {
        let pool = pool.clone();
        let cancel = cancel.clone();
        tokio::spawn(scheduler::run(initial_monitors, pool, cancel, monitor_rx))
    };

    let web_handle = {
        let bind = cfg.web.bind.clone();
        let cancel = cancel.clone();
        tokio::spawn(web::serve(bind, state, cancel))
    };

    let retention_handle = if let Some(days) = cfg.defaults.retention_days {
        let pool = pool.clone();
        let cancel = cancel.clone();
        Some(tokio::spawn(async move {
            scheduler::retention_loop(pool, days, cancel).await;
        }))
    } else {
        None
    };

    #[cfg(not(windows))]
    if is_first_launch {
        let url = dashboard_url.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(800)).await;
            let _ = webbrowser::open(&url);
        });
    }
    let _ = (is_first_launch, dashboard_url);

    tokio::select! {
        _ = tokio::signal::ctrl_c() => { info!("Received shutdown signal"); }
        _ = cancel.cancelled() => { info!("Shutdown requested"); }
    }
    cancel.cancel();

    let drain = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        let _ = sched_handle.await;
        let _ = web_handle.await;
        if let Some(h) = retention_handle { let _ = h.await; }
    });
    if drain.await.is_err() { tracing::warn!("Drain timeout exceeded"); }

    info!("Shutdown complete");
    Ok(())
}

/// If the raw config.toml has `[[monitors]]` entries (legacy format), migrate
/// them to monitors.toml. Runs before the config is parsed, since the current
/// schema cannot represent the legacy array-of-tables.
fn migrate_legacy_config(config_path: &std::path::Path, monitors_path: &std::path::Path) {
    // Don't clobber an existing monitors.toml.
    if monitors_path.exists() || !config_path.exists() {
        return;
    }

    // Parse raw config, extracting only legacy `[[monitors]]` entries.
    #[derive(serde::Deserialize, Default)]
    struct LegacyConfig {
        #[serde(default)]
        monitors: Vec<monitors::MonitorConfig>,
    }

    let raw = match std::fs::read_to_string(config_path) {
        Ok(s) => s,
        Err(_) => return,
    };
    let legacy: LegacyConfig = match toml::from_str(&raw) {
        Ok(c) => c,
        Err(_) => return,
    };

    if legacy.monitors.is_empty() {
        return;
    }

    if let Err(e) = monitors::migrate_legacy_monitors(legacy.monitors, monitors_path, config_path) {
        tracing::error!(error = %e, "Migration failed");
    }
}

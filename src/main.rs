// Suppress the console window on Windows release builds.
// Debug builds keep the console so log output is visible during development.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use rusty_pingus::{config, db, probe, scheduler, web};
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
    /// Path to config file
    #[arg(long, default_value = "./config.toml")]
    pub config: PathBuf,

    /// Override web server bind address (e.g. 0.0.0.0:3000)
    #[arg(long)]
    pub bind: Option<String>,

    /// Override database file path
    #[arg(long)]
    pub db: Option<String>,
}

fn dashboard_url(bind: &str) -> String {
    format!("http://{}", bind.replace("0.0.0.0", "localhost"))
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut cfg = config::load(&cli.config)?;
    if let Some(bind) = cli.bind { cfg.web.bind = bind; }
    if let Some(db_path) = cli.db { cfg.database.path = db_path; }

    if let Some(parent) = std::path::Path::new(&cfg.database.path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Detect first launch before the DB file is created.
    let is_first_launch = !std::path::Path::new(&cfg.database.path).exists();
    let url = dashboard_url(&cfg.web.bind);

    // ── Logging init ─────────────────────────────────────────────────────────
    // On Windows release builds: write to a rolling log file (no console).
    // All other cases: write to stdout.

    #[cfg(all(windows, not(debug_assertions)))]
    let _log_guard = {
        let log_dir = {
            let db_parent = std::path::Path::new(&cfg.database.path)
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            db_parent.join("logs")
        };
        std::fs::create_dir_all(&log_dir).ok();
        let file_appender = tracing_appender::rolling::daily(&log_dir, "rusty-pingus.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .with_writer(non_blocking)
            .with_ansi(false)
            .init();
        guard
    };

    #[cfg(not(all(windows, not(debug_assertions))))]
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // ── Platform entry ────────────────────────────────────────────────────────

    #[cfg(windows)]
    {
        let cancel = CancellationToken::new();

        // Spawn the Tokio runtime on a background thread so the main thread
        // is free to run the tray event loop (Win32 requirement).
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

        // Give the server a moment to bind before the tray appears.
        std::thread::sleep(std::time::Duration::from_millis(800));

        // Auto-open browser on first launch.
        if is_first_launch {
            let _ = webbrowser::open(&url);
        }

        // Main thread: run tray event loop — never returns on Windows.
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
    let pool = db::init(&cfg.database.path).await?;

    info!("Starting rusty-pingus with {} monitors", cfg.monitors.len());

    let has_icmp = cfg.monitors.iter().any(|m| matches!(m, config::MonitorConfig::Icmp(_)));
    if has_icmp && !probe::icmp::check_privilege() {
        tracing::warn!("ICMP monitors configured but CAP_NET_RAW may be unavailable.");
    }

    let sched_handle = {
        let pool = pool.clone();
        let monitors = cfg.monitors.clone();
        let cancel = cancel.clone();
        tokio::spawn(scheduler::run(monitors, pool, cancel))
    };

    let web_handle = {
        let pool = pool.clone();
        let bind = cfg.web.bind.clone();
        let cancel = cancel.clone();
        tokio::spawn(web::serve(bind, pool, cancel))
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

    // On non-Windows first launch, open the browser after a short delay.
    #[cfg(not(windows))]
    if is_first_launch {
        let url = dashboard_url.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(800)).await;
            let _ = webbrowser::open(&url);
        });
    }

    // Suppress unused warning on Windows (browser opened before async_main on that path).
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
    if drain.await.is_err() {
        tracing::warn!("Drain timeout exceeded");
    }

    info!("Shutdown complete");
    Ok(())
}

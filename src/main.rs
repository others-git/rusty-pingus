use rusty_pingus::{config, db, probe, scheduler, web};

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
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

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    let mut cfg = config::load(&cli.config)?;

    if let Some(bind) = cli.bind {
        cfg.web.bind = bind;
    }
    if let Some(db_path) = cli.db {
        cfg.database.path = db_path;
    }

    if let Some(parent) = std::path::Path::new(&cfg.database.path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let pool = db::init(&cfg.database.path).await?;

    info!("Starting rusty-pingus with {} monitors", cfg.monitors.len());

    let has_icmp = cfg.monitors.iter().any(|m| matches!(m, config::MonitorConfig::Icmp(_)));
    if has_icmp && !probe::icmp::check_privilege() {
        tracing::warn!("ICMP monitors configured but CAP_NET_RAW may be unavailable — ICMP probes may fail. Run with sudo or set CAP_NET_RAW.");
    }

    let cancel = tokio_util::sync::CancellationToken::new();

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

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
    }

    cancel.cancel();

    let drain = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        let _ = sched_handle.await;
        let _ = web_handle.await;
        if let Some(h) = retention_handle {
            let _ = h.await;
        }
    });
    if drain.await.is_err() {
        tracing::warn!("Drain timeout exceeded — forcing exit");
    }

    info!("Shutdown complete");
    Ok(())
}

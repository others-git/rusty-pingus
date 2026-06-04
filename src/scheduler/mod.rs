use std::time::Duration;
use sqlx::SqlitePool;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::config::MonitorConfig;
use crate::db;
use crate::probe;

pub async fn run(monitors: Vec<MonitorConfig>, pool: SqlitePool, cancel: CancellationToken) {
    let mut handles = Vec::new();

    for monitor in monitors {
        let pool = pool.clone();
        let cancel = cancel.clone();
        let handle = tokio::spawn(monitor_loop(monitor, pool, cancel));
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.await;
    }
}

async fn monitor_loop(monitor: MonitorConfig, pool: SqlitePool, cancel: CancellationToken) {
    let name = monitor.name().to_string();
    let interval_secs = monitor.interval_secs();
    let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                run_probe_with_restart(&monitor, &pool, &name).await;
            }
            _ = cancel.cancelled() => {
                info!(monitor = %name, "Scheduler stopping");
                break;
            }
        }
    }
}

async fn run_probe_with_restart(monitor: &MonitorConfig, pool: &SqlitePool, name: &str) {
    match run_probe(monitor, pool, name).await {
        Ok(()) => {}
        Err(e) => error!(monitor = %name, error = %e, "Probe task error"),
    }
}

async fn run_probe(
    monitor: &MonitorConfig,
    pool: &SqlitePool,
    _name: &str,
) -> anyhow::Result<()> {
    let result = match monitor {
        MonitorConfig::Http(cfg) => probe::http::run(cfg).await,
        MonitorConfig::Tcp(cfg) => probe::tcp::run(cfg).await,
        MonitorConfig::Icmp(cfg) => probe::icmp::run(cfg).await,
    };

    if let Err(e) = db::insert_result(pool, &result).await {
        warn!(error = %e, "Failed to persist probe result — continuing");
    }
    Ok(())
}

pub async fn retention_loop(pool: SqlitePool, retention_days: u64, cancel: CancellationToken) {
    let mut ticker = tokio::time::interval(Duration::from_secs(86_400));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                match db::prune_old_results(&pool, retention_days).await {
                    Ok(n) => info!(deleted = n, "Pruned old probe results"),
                    Err(e) => error!(error = %e, "Retention cleanup failed"),
                }
            }
            _ = cancel.cancelled() => break,
        }
    }
}

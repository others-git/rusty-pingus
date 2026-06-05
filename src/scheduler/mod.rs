use std::collections::HashMap;
use std::time::Duration;
use sqlx::SqlitePool;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::monitors::MonitorConfig;
use crate::db;
use crate::probe;

pub async fn run(
    initial_monitors: Vec<MonitorConfig>,
    pool: SqlitePool,
    cancel: CancellationToken,
    mut monitor_rx: watch::Receiver<Vec<MonitorConfig>>,
) {
    // Track per-monitor cancel tokens so we can stop individual tasks.
    let mut task_tokens: HashMap<String, CancellationToken> = HashMap::new();

    // Spawn initial tasks
    for monitor in &initial_monitors {
        let token = spawn_monitor(monitor.clone(), pool.clone(), cancel.clone());
        task_tokens.insert(monitor.name().to_string(), token);
    }

    // Watch for monitor list changes (hot-reload) or global shutdown
    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("Scheduler shutting down");
                // Cancel all running tasks
                for token in task_tokens.values() {
                    token.cancel();
                }
                break;
            }
            Ok(()) = monitor_rx.changed() => {
                let new_monitors = monitor_rx.borrow_and_update().clone();
                diff_and_reload(&mut task_tokens, &new_monitors, &pool, &cancel);
            }
        }
    }
}

fn diff_and_reload(
    task_tokens: &mut HashMap<String, CancellationToken>,
    new_monitors: &[MonitorConfig],
    pool: &SqlitePool,
    global_cancel: &CancellationToken,
) {
    let new_names: std::collections::HashSet<String> =
        new_monitors.iter().map(|m| m.name().to_string()).collect();
    // Collect old names up front to avoid borrow conflict
    let old_names: Vec<String> = task_tokens.keys().cloned().collect();

    // Cancel tasks for removed monitors
    for name in &old_names {
        if !new_names.contains(name) {
            if let Some(token) = task_tokens.remove(name) {
                info!(monitor = %name, "Stopping removed monitor");
                token.cancel();
            }
        }
    }

    let old_name_set: std::collections::HashSet<&str> =
        old_names.iter().map(|s| s.as_str()).collect();

    // Spawn tasks for added monitors
    for monitor in new_monitors {
        if !old_name_set.contains(monitor.name()) {
            info!(monitor = %monitor.name(), "Starting new monitor");
            let token = spawn_monitor(monitor.clone(), pool.clone(), global_cancel.clone());
            task_tokens.insert(monitor.name().to_string(), token);
        }
    }
}

fn spawn_monitor(
    monitor: MonitorConfig,
    pool: SqlitePool,
    global_cancel: CancellationToken,
) -> CancellationToken {
    let task_cancel = CancellationToken::new();
    let task_cancel_clone = task_cancel.clone();

    tokio::spawn(async move {
        let name = monitor.name().to_string();
        let interval_ms = monitor.interval_ms();
        let mut ticker = tokio::time::interval(Duration::from_millis(interval_ms));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    match run_probe(&monitor, &pool).await {
                        Ok(()) => {}
                        Err(e) => error!(monitor = %name, error = %e, "Probe error"),
                    }
                }
                _ = task_cancel_clone.cancelled() => {
                    info!(monitor = %name, "Monitor task stopped");
                    break;
                }
                _ = global_cancel.cancelled() => {
                    break;
                }
            }
        }
    });

    task_cancel
}

async fn run_probe(monitor: &MonitorConfig, pool: &SqlitePool) -> anyhow::Result<()> {
    let result = match monitor {
        MonitorConfig::Http(cfg) => probe::http::run(cfg).await,
        MonitorConfig::Tcp(cfg) => probe::tcp::run(cfg).await,
        MonitorConfig::Icmp(cfg) => probe::icmp::run(cfg).await,
    };
    if let Err(e) = db::insert_result(pool, &result).await {
        warn!(error = %e, "Failed to persist probe result");
    }
    Ok(())
}

pub async fn retention_loop(pool: SqlitePool, retention_days: u64, cancel: CancellationToken) {
    info!(retention_days, "Retention loop started; pruning probe results older than this daily");
    let mut ticker = tokio::time::interval(Duration::from_secs(86_400));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                match db::prune_old_results(&pool, retention_days).await {
                    Ok(n) => info!(deleted = n, "Pruned old probe results"),
                    Err(e) => error!(error = %e, "Retention cleanup failed"),
                }
                if let Err(e) = db::prune_old_rollups(&pool, retention_days).await {
                    error!(error = %e, "Rollup retention cleanup failed");
                }
            }
            _ = cancel.cancelled() => break,
        }
    }
}

/// Keep the per-minute rollups current: on start, backfill from the rollup
/// watermark (or earliest raw minute) up to now; then every ~30s roll up newly
/// completed minutes. The current (incomplete) minute is excluded; the boundary
/// minute is re-rolled via INSERT OR REPLACE so partials are corrected.
pub async fn rollup_loop(pool: SqlitePool, cancel: CancellationToken) {
    let current_minute = || (chrono::Utc::now().timestamp() / 60) * 60;

    let mut next_from = match db::rollup_watermark(&pool).await {
        Ok(Some(w)) => w, // re-roll from the last rolled minute (REPLACE handles partials/gaps)
        Ok(None) => match db::earliest_raw_minute(&pool).await {
            Ok(Some(m)) => m,
            _ => current_minute(),
        },
        Err(e) => {
            error!(error = %e, "Rollup watermark read failed");
            current_minute()
        }
    };

    let mut ticker = tokio::time::interval(Duration::from_secs(30));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    info!(start = next_from, "Rollup maintenance loop started");

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let now_min = current_minute();
                // Roll complete minutes [next_from, now_min) in day-sized chunks.
                let mut from = next_from;
                while from < now_min {
                    let to = (from + 86_400).min(now_min);
                    match db::roll_up_range(&pool, from, to).await {
                        Ok(_) => { from = to; }
                        Err(e) => { error!(error = %e, "Rollup aggregation failed"); break; }
                    }
                }
                next_from = now_min;
            }
            _ = cancel.cancelled() => break,
        }
    }
}

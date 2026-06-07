use std::collections::HashMap;
use std::time::Duration;
use sqlx::SqlitePool;
use tokio::sync::{broadcast, watch};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::api::StatusUpdate;
use crate::monitors::{MonitorConfig, MonitorStore};
use crate::db;
use crate::probe;

pub async fn run(
    initial_monitors: Vec<MonitorConfig>,
    pool: SqlitePool,
    cancel: CancellationToken,
    mut monitor_rx: watch::Receiver<Vec<MonitorConfig>>,
    updates: broadcast::Sender<StatusUpdate>,
) {
    // Track per-monitor cancel tokens so we can stop individual tasks.
    let mut task_tokens: HashMap<String, CancellationToken> = HashMap::new();

    // Spawn initial tasks (disabled monitors are not probed).
    for monitor in &initial_monitors {
        if !monitor.enabled() {
            info!(monitor = %monitor.name(), "Monitor disabled; not scheduling");
            continue;
        }
        let token = spawn_monitor(monitor.clone(), pool.clone(), cancel.clone(), updates.clone());
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
                diff_and_reload(&mut task_tokens, &new_monitors, &pool, &cancel, &updates);
            }
        }
    }
}

fn diff_and_reload(
    task_tokens: &mut HashMap<String, CancellationToken>,
    new_monitors: &[MonitorConfig],
    pool: &SqlitePool,
    global_cancel: &CancellationToken,
    updates: &broadcast::Sender<StatusUpdate>,
) {
    // `task_tokens` holds only *running* tasks; the desired set is the enabled
    // monitors. Reconcile the two so this path also handles enable/disable
    // toggles (not just add/remove): a monitor that became disabled has its task
    // cancelled, and one that became enabled gets a task spawned.
    let old_names: Vec<String> = task_tokens.keys().cloned().collect();

    // Stop tasks for monitors that were removed entirely or are now disabled.
    for name in &old_names {
        let still_wanted = new_monitors.iter().any(|m| m.name() == name && m.enabled());
        if !still_wanted {
            if let Some(token) = task_tokens.remove(name) {
                info!(monitor = %name, "Stopping monitor (removed or disabled)");
                token.cancel();
            }
        }
    }

    // Start tasks for enabled monitors that don't have one yet (added or enabled).
    for monitor in new_monitors {
        if monitor.enabled() && !task_tokens.contains_key(monitor.name()) {
            info!(monitor = %monitor.name(), "Starting monitor");
            let token = spawn_monitor(monitor.clone(), pool.clone(), global_cancel.clone(), updates.clone());
            task_tokens.insert(monitor.name().to_string(), token);
        }
    }
}

fn spawn_monitor(
    monitor: MonitorConfig,
    pool: SqlitePool,
    global_cancel: CancellationToken,
    updates: broadcast::Sender<StatusUpdate>,
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
                    match run_probe(&monitor, &pool, &updates).await {
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

async fn run_probe(
    monitor: &MonitorConfig,
    pool: &SqlitePool,
    updates: &broadcast::Sender<StatusUpdate>,
) -> anyhow::Result<()> {
    let result = match monitor {
        MonitorConfig::Http(cfg) => probe::http::run(cfg).await,
        MonitorConfig::Tcp(cfg) => probe::tcp::run(cfg).await,
        MonitorConfig::Icmp(cfg) => probe::icmp::run(cfg).await,
        MonitorConfig::PublicIp(cfg) => probe::publicip::run(cfg, pool).await,
        MonitorConfig::Border(cfg) => probe::border::run(cfg).await,
        MonitorConfig::Traceroute(cfg) => {
            // Traceroute yields both a dashboard summary and per-hop data. Persist
            // the hop data to the dedicated tables; the summary goes to probe_results
            // below like every other monitor.
            let (summary, run) = probe::traceroute::run(cfg).await;
            if let Some(run) = run {
                if let Err(e) = db::insert_traceroute(pool, &summary.monitor_name, summary.checked_at, &run).await {
                    warn!(monitor = %summary.monitor_name, error = %e, "Failed to persist traceroute run");
                }
            }
            summary
        }
    };
    if let Err(e) = db::insert_result(pool, &result).await {
        warn!(error = %e, "Failed to persist probe result");
    }
    // Push a live status update to any connected dashboards. Built directly from
    // the ProbeResult (no extra query); a send error just means no subscribers.
    let _ = updates.send(StatusUpdate::from(&result));
    Ok(())
}

/// Per-monitor retention loop: runs hourly, reads the live monitor list, and
/// prunes each monitor's probe_results + rollups (and traceroute hops) to its
/// own `retention_hours` setting. Picks up config changes without a restart.
pub async fn per_monitor_retention_loop(
    pool: SqlitePool,
    monitors: MonitorStore,
    global_retention_days: u64,
    cancel: CancellationToken,
) {
    info!(global_retention_days, "Per-monitor retention loop started");
    let mut ticker = tokio::time::interval(Duration::from_secs(3_600));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                for monitor in monitors.list().await {
                    let name = monitor.name().to_string();
                    let retention_hours = monitor.retention_hours(global_retention_days);
                    let cutoff_ms = (chrono::Utc::now()
                        - chrono::Duration::hours(retention_hours as i64))
                        .timestamp_millis();
                    let cutoff_epoch = cutoff_ms / 1000;

                    match db::prune_monitor_results(&pool, &name, cutoff_ms).await {
                        Ok(n) if n > 0 => info!(monitor = %name, deleted = n, retention_hours, "Pruned old probe results"),
                        Ok(_) => {}
                        Err(e) => error!(monitor = %name, error = %e, "Probe result prune failed"),
                    }
                    if let Err(e) = db::prune_monitor_rollups(&pool, &name, cutoff_epoch).await {
                        error!(monitor = %name, error = %e, "Rollup prune failed");
                    }
                    if matches!(monitor, MonitorConfig::Traceroute(_)) {
                        let retention_ms = retention_hours.saturating_mul(3_600_000);
                        match db::prune_traceroute(&pool, &name, retention_ms).await {
                            Ok(n) if n > 0 => info!(monitor = %name, deleted = n, "Pruned old traceroute runs"),
                            Ok(_) => {}
                            Err(e) => error!(monitor = %name, error = %e, "Traceroute prune failed"),
                        }
                    }
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

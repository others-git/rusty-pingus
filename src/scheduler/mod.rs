use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use sqlx::SqlitePool;
use tokio::sync::{broadcast, watch};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::api::StatusUpdate;
use crate::monitors::{MonitorConfig, MonitorStore, StoredMonitor};
use crate::db;
use crate::probe;

pub async fn run(
    initial_monitors: Arc<Vec<StoredMonitor>>,
    pool: SqlitePool,
    cancel: CancellationToken,
    mut monitor_rx: watch::Receiver<Arc<Vec<StoredMonitor>>>,
    updates: broadcast::Sender<StatusUpdate>,
) {
    // Track running tasks by stable id. The stored string is the serialized
    // config ("version"): comparing it on reload lets us restart only the tasks
    // whose config actually changed, leaving the rest (and their tickers) alone.
    let mut task_tokens: HashMap<i64, (CancellationToken, String)> = HashMap::new();

    // Spawn initial tasks (disabled monitors are not probed).
    for monitor in initial_monitors.iter() {
        if !monitor.config.enabled() {
            info!(monitor = %monitor.config.name(), "Monitor disabled; not scheduling");
            continue;
        }
        let token = spawn_monitor(monitor.clone(), pool.clone(), cancel.clone(), updates.clone());
        task_tokens.insert(monitor.id, (token, config_key(&monitor.config)));
    }

    // Watch for monitor list changes (hot-reload) or global shutdown
    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("Scheduler shutting down");
                // Cancel all running tasks
                for (token, _) in task_tokens.values() {
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
    task_tokens: &mut HashMap<i64, (CancellationToken, String)>,
    new_monitors: &[StoredMonitor],
    pool: &SqlitePool,
    global_cancel: &CancellationToken,
    updates: &broadcast::Sender<StatusUpdate>,
) {
    // `task_tokens` holds only *running* tasks; the desired set is the enabled
    // monitors. Reconcile the two so this path handles add/remove, enable/disable
    // toggles, and edits — all keyed by stable id, so a renamed monitor keeps its
    // running task (and history) untouched.
    let old_ids: Vec<i64> = task_tokens.keys().copied().collect();

    // Stop tasks for monitors that were removed entirely or are now disabled.
    for id in &old_ids {
        let still_wanted = new_monitors.iter().any(|m| m.id == *id && m.config.enabled());
        if !still_wanted {
            if let Some((token, _)) = task_tokens.remove(id) {
                info!(monitor_id = id, "Stopping monitor (removed or disabled)");
                token.cancel();
            }
        }
    }

    // Start tasks for enabled monitors that have no task yet, and restart those
    // whose config changed (e.g. interval/host edited) so the probe picks it up.
    for monitor in new_monitors {
        if !monitor.config.enabled() {
            continue;
        }
        let key = config_key(&monitor.config);
        match task_tokens.get(&monitor.id) {
            Some((_, current)) if *current == key => continue, // unchanged; leave running
            Some((token, _)) => {
                info!(monitor = %monitor.config.name(), "Restarting monitor (config changed)");
                token.cancel();
            }
            None => info!(monitor = %monitor.config.name(), "Starting monitor"),
        }
        let token = spawn_monitor(monitor.clone(), pool.clone(), global_cancel.clone(), updates.clone());
        task_tokens.insert(monitor.id, (token, key));
    }
}

/// Serialized config used as a cheap change-detector for hot-reload: if it
/// differs from the running task's, the task is restarted.
fn config_key(config: &MonitorConfig) -> String {
    serde_json::to_string(config).unwrap_or_default()
}

fn spawn_monitor(
    monitor: StoredMonitor,
    pool: SqlitePool,
    global_cancel: CancellationToken,
    updates: broadcast::Sender<StatusUpdate>,
) -> CancellationToken {
    let task_cancel = CancellationToken::new();
    let task_cancel_clone = task_cancel.clone();

    tokio::spawn(async move {
        let id = monitor.id;
        let config = monitor.config;
        let name = config.name().to_string();
        let interval_ms = config.interval_ms();

        // Stagger the first tick: tickers otherwise start together, so monitors
        // sharing an interval probe (and write) in lockstep forever. The offset
        // is deterministic per id and capped at 5 s so the first probe still
        // lands promptly after startup.
        let jitter_ms = (id as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) % interval_ms.clamp(1, 5_000);
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(jitter_ms)) => {}
            _ = task_cancel_clone.cancelled() => return,
            _ = global_cancel.cancelled() => return,
        }

        let mut ticker = tokio::time::interval(Duration::from_millis(interval_ms));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    match run_probe(&config, id, &pool, &updates).await {
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
    monitor_id: i64,
    pool: &SqlitePool,
    updates: &broadcast::Sender<StatusUpdate>,
) -> anyhow::Result<()> {
    let result = match monitor {
        MonitorConfig::Http(cfg) => probe::http::run(cfg).await,
        MonitorConfig::Tcp(cfg) => probe::tcp::run(cfg).await,
        MonitorConfig::Icmp(cfg) => probe::icmp::run(cfg).await,
        MonitorConfig::PublicIp(cfg) => probe::publicip::run(cfg, pool, monitor_id).await,
        MonitorConfig::Border(cfg) => probe::border::run(cfg).await,
        MonitorConfig::Traceroute(cfg) => {
            // Traceroute yields both a dashboard summary and per-hop data. Persist
            // the hop data to the dedicated tables; the summary goes to probe_results
            // below like every other monitor.
            let (summary, run) = probe::traceroute::run(cfg).await;
            if let Some(run) = run {
                if let Err(e) = db::insert_traceroute(pool, monitor_id, &summary.monitor_name, summary.checked_at, &run).await {
                    warn!(monitor = %summary.monitor_name, error = %e, "Failed to persist traceroute run");
                }
            }
            summary
        }
    };
    if let Err(e) = db::insert_result(pool, monitor_id, &result).await {
        warn!(error = %e, "Failed to persist probe result");
    }
    // Push a live status update to any connected dashboards. Built directly from
    // the ProbeResult (no extra query); a send error just means no subscribers.
    let _ = updates.send(StatusUpdate::from_result(monitor_id, &result));
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
                for monitor in monitors.list().await.iter() {
                    let id = monitor.id;
                    let name = monitor.config.name().to_string();
                    let retention_hours = monitor.config.retention_hours(global_retention_days);
                    let cutoff_ms = (chrono::Utc::now()
                        - chrono::Duration::hours(retention_hours as i64))
                        .timestamp_millis();
                    let cutoff_epoch = cutoff_ms / 1000;

                    match db::prune_monitor_results(&pool, id, cutoff_ms).await {
                        Ok(n) if n > 0 => info!(monitor = %name, deleted = n, retention_hours, "Pruned old probe results"),
                        Ok(_) => {}
                        Err(e) => error!(monitor = %name, error = %e, "Probe result prune failed"),
                    }
                    if let Err(e) = db::prune_monitor_rollups(&pool, id, cutoff_epoch).await {
                        error!(monitor = %name, error = %e, "Rollup prune failed");
                    }
                    if matches!(monitor.config, MonitorConfig::Traceroute(_)) {
                        let retention_ms = retention_hours.saturating_mul(3_600_000);
                        match db::prune_traceroute(&pool, id, retention_ms).await {
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
                // Resume from where rolling actually stopped: on a failed chunk
                // this retries the same range next tick instead of skipping it
                // (a skipped range would never be re-rolled — the watermark
                // moves past it).
                next_from = from;
            }
            _ = cancel.cancelled() => break,
        }
    }
}

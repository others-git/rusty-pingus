use rusty_pingus::{config, monitors::TcpMonitorConfig, db, probe};
use std::time::Duration;
use tokio::net::TcpListener;

async fn open_temp_db() -> (sqlx::SqlitePool, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("test.db");
    let pool = db::init(path.to_str().unwrap()).await.expect("db::init");
    (pool, dir)
}

/// Bind a random loopback port, return its address.
async fn loopback_listener() -> (TcpListener, std::net::SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    (listener, addr)
}

#[tokio::test]
async fn tcp_probe_up_stored_in_db() {
    let (pool, _dir) = open_temp_db().await;

    // Bind a local listener so the TCP probe has something to connect to.
    let (listener, addr) = loopback_listener().await;
    tokio::spawn(async move {
        // Accept one connection then drop it — enough for the probe to succeed.
        let _ = listener.accept().await;
    });

    let cfg = TcpMonitorConfig {
        name: "test-tcp".into(),
        host: "127.0.0.1".into(),
        port: addr.port(),
        interval_ms: 60_000,
        timeout_ms: 5_000,
        retention_hours: None,
        enabled: true,
    };

    let result = probe::tcp::run(&cfg).await;
    assert_eq!(result.status, "up", "expected probe to succeed against local listener");
    assert!(result.response_time_ms.is_some());

    db::insert_result(&pool, &result).await.expect("insert_result");

    let statuses = db::get_current_status(&pool).await.expect("get_current_status");
    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0].monitor_name, "test-tcp");
    assert_eq!(statuses[0].status, "up");
}

#[tokio::test]
async fn tcp_probe_down_stored_in_db() {
    let (pool, _dir) = open_temp_db().await;

    // Bind a random port to get a free one, then immediately drop the listener.
    // The OS will refuse connections to that port immediately on both Linux and
    // Windows — unlike port 1 which may hit a firewall timeout on Windows.
    let refused_port = {
        let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let p = l.local_addr().unwrap().port();
        drop(l);
        p
    };

    let cfg = TcpMonitorConfig {
        name: "test-tcp-down".into(),
        host: "127.0.0.1".into(),
        port: refused_port,
        interval_ms: 60_000,
        timeout_ms: 2_000,
        retention_hours: None,
        enabled: true,
    };

    let result = probe::tcp::run(&cfg).await;
    assert_eq!(result.status, "down");
    assert!(result.failure_reason.is_some());

    db::insert_result(&pool, &result).await.expect("insert_result");

    let statuses = db::get_current_status(&pool).await.expect("get_current_status");
    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0].status, "down");
}

#[tokio::test]
async fn detail_round_trips_through_insert_and_history() {
    let (pool, _dir) = open_temp_db().await;

    // An up result carrying type-specific detail (e.g. a public-IP reading).
    let mut with = probe::ProbeResult::up("ip-mon", "publicip", "https://checkip.amazonaws.com", 12)
        .with_detail("203.0.113.7");
    // A plain result with no detail, recorded a minute earlier so ordering is stable.
    let mut without = probe::ProbeResult::up("ip-mon", "publicip", "https://checkip.amazonaws.com", 9);
    without.checked_at = chrono::Utc::now() - chrono::Duration::seconds(60);
    with.checked_at = chrono::Utc::now();
    db::insert_result(&pool, &without).await.expect("insert without");
    db::insert_result(&pool, &with).await.expect("insert with");

    let history = db::get_history(&pool, "ip-mon", None, None, 10)
        .await
        .expect("get_history");
    assert_eq!(history.len(), 2);
    // Newest first: the detail-bearing row round-trips its value.
    assert_eq!(history[0].detail.as_deref(), Some("203.0.113.7"));
    assert_eq!(history[1].detail, None);

    // Current status also carries the latest detail.
    let latest = db::get_latest_status(&pool, "ip-mon")
        .await
        .expect("get_latest_status")
        .expect("a row");
    assert_eq!(latest.detail.as_deref(), Some("203.0.113.7"));
}

#[tokio::test]
async fn uptime_calculation_correct() {
    let (pool, _dir) = open_temp_db().await;

    // Insert 3 up, 1 down → 75% uptime.
    for i in 0..4u32 {
        let r = probe::ProbeResult {
            monitor_name: "uptime-test".into(),
            protocol: "tcp".into(),
            endpoint: "127.0.0.1:9999".into(),
            status: if i < 3 { "up" } else { "down" }.into(),
            response_time_ms: if i < 3 { Some(10) } else { None },
            failure_reason: if i < 3 { None } else { Some("timeout".into()) },
            detail: None,
            checked_at: chrono::Utc::now()
                - chrono::Duration::seconds((3 - i as i64) * 10),
        };
        db::insert_result(&pool, &r).await.expect("insert");
    }

    let uptime = db::get_uptime(&pool, "uptime-test", 3600)
        .await
        .expect("get_uptime");

    assert!(uptime.is_some());
    let pct = uptime.unwrap();
    assert!((pct - 75.0).abs() < 0.01, "expected 75.0%, got {pct}");
}

#[tokio::test]
async fn history_query_returns_ordered_results() {
    let (pool, _dir) = open_temp_db().await;

    for i in 0..5i64 {
        let r = probe::ProbeResult {
            monitor_name: "hist-test".into(),
            protocol: "http".into(),
            endpoint: "https://example.com".into(),
            status: "up".into(),
            response_time_ms: Some((i * 10) as u64),
            failure_reason: None,
            detail: None,
            checked_at: chrono::Utc::now() - chrono::Duration::seconds(i * 60),
        };
        db::insert_result(&pool, &r).await.expect("insert");
    }

    let history = db::get_history(&pool, "hist-test", None, None, 10)
        .await
        .expect("get_history");

    assert_eq!(history.len(), 5);
    // Newest first
    for w in history.windows(2) {
        assert!(w[0].checked_at >= w[1].checked_at, "results should be newest-first");
    }
}

#[tokio::test]
async fn series_buckets_are_bounded_and_aggregated() {
    let (pool, _dir) = open_temp_db().await;
    let now = chrono::Utc::now();

    // 20 results over the last hour (every ~3 min); every 4th is down.
    for i in 0..20i64 {
        let up = i % 4 != 0;
        let r = probe::ProbeResult {
            monitor_name: "series-test".into(),
            protocol: "http".into(),
            endpoint: "https://example.com".into(),
            status: if up { "up" } else { "down" }.into(),
            response_time_ms: if up { Some((10 + i) as u64) } else { None },
            failure_reason: if up { None } else { Some("timeout".into()) },
            detail: None,
            checked_at: now - chrono::Duration::minutes((19 - i) * 3),
        };
        db::insert_result(&pool, &r).await.expect("insert");
    }

    let from = now - chrono::Duration::hours(1) - chrono::Duration::minutes(5);
    let buckets = db::get_series(&pool, "series-test", from, now, 5)
        .await
        .expect("get_series");

    assert!(!buckets.is_empty(), "expected some buckets");
    assert!(buckets.len() <= 5, "expected at most 5 buckets, got {}", buckets.len());

    let total: i64 = buckets.iter().map(|b| b.count).sum();
    assert_eq!(total, 20, "all 20 results should be counted across buckets");

    for b in &buckets {
        assert!(b.count > 0, "each returned bucket has rows");
        assert!(b.up_ratio >= 0.0 && b.up_ratio <= 1.0, "up_ratio in [0,1], got {}", b.up_ratio);
    }

    // A range with no results returns an empty series.
    let empty = db::get_series(
        &pool,
        "series-test",
        now - chrono::Duration::days(40),
        now - chrono::Duration::days(39),
        5,
    )
    .await
    .expect("get_series empty");
    assert!(empty.is_empty(), "no results in range → empty series");
}

#[tokio::test]
async fn rollups_back_series_and_uptime() {
    let (pool, _dir) = open_temp_db().await;
    let now = chrono::Utc::now();

    // Seed 5 distinct minutes; each minute has 2 up (10ms, 20ms) + 1 down → per
    // minute: count=3, up=2, up_ratio=2/3. Total: 15 probes, 10 up.
    let minute_offsets = [80i64, 60, 40, 20, 5];
    for off in minute_offsets {
        let base = now - chrono::Duration::minutes(off);
        for (i, rt) in [Some(10u64), Some(20u64), None].iter().enumerate() {
            let r = probe::ProbeResult {
                monitor_name: "roll".into(),
                protocol: "tcp".into(),
                endpoint: "h:1".into(),
                status: if rt.is_some() { "up" } else { "down" }.into(),
                response_time_ms: *rt,
                failure_reason: if rt.is_some() { None } else { Some("timeout".into()) },
                detail: None,
                checked_at: base + chrono::Duration::seconds(i as i64), // same minute
            };
            db::insert_result(&pool, &r).await.expect("insert");
        }
    }

    // Roll up the whole span.
    let from = (now - chrono::Duration::minutes(90)).timestamp();
    let to = now.timestamp() + 60;
    db::roll_up_range(&pool, from, to).await.expect("roll_up_range");

    assert!(db::rollup_watermark(&pool).await.expect("watermark").is_some());

    // get_series over a wide span (span/buckets >= 60) → served from the rollup.
    let series = db::get_series(
        &pool,
        "roll",
        now - chrono::Duration::minutes(90),
        now,
        5,
    )
    .await
    .expect("get_series");
    let total: i64 = series.iter().map(|b| b.count).sum();
    assert_eq!(total, 15, "rollup-backed series should count all 15 probes");
    for b in &series {
        assert!((b.up_ratio - 2.0 / 3.0).abs() < 1e-9, "up_ratio 2/3, got {}", b.up_ratio);
        assert_eq!(b.avg_ms, Some(15.0), "avg of 10 and 20 = 15");
        assert_eq!(b.min_ms, Some(10));
        assert_eq!(b.max_ms, Some(20));
    }

    // get_uptime over a 30d window (>24h) → served from the rollup.
    let uptime = db::get_uptime(&pool, "roll", 2_592_000).await.expect("uptime").unwrap();
    assert!((uptime - (10.0 / 15.0) * 100.0).abs() < 0.01, "expected 66.67%, got {uptime}");

    // delete_results also clears rollups.
    db::delete_results(&pool, "roll").await.expect("delete");
    assert!(db::rollup_watermark(&pool).await.expect("watermark").is_none());
}

#[test]
fn missing_config_generates_default_and_returns_empty() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");

    assert!(!config_path.exists(), "precondition: file should not exist");

    let cfg = config::load(&config_path).expect("load should succeed with missing file");

    // File should now be written
    assert!(config_path.exists(), "default config should have been written");

    // monitors field is now MonitorsConfig (path), not a Vec
    assert!(!cfg.monitors.path.is_empty(), "monitors path should be set");
    assert_eq!(cfg.web.bind, "0.0.0.0:3000");

    // The written file should be valid TOML that parses without error
    let reloaded = config::load(&config_path).expect("reloaded default config should be valid");
    assert!(!reloaded.monitors.path.is_empty());
}

#[test]
fn existing_config_not_overwritten() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");

    // Write a custom config
    std::fs::write(&config_path, "[web]\nbind = \"127.0.0.1:9999\"\n").unwrap();

    let cfg = config::load(&config_path).expect("load");
    assert_eq!(cfg.web.bind, "127.0.0.1:9999", "existing config should be respected");
}

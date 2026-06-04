use rusty_pingus::{config::TcpMonitorConfig, db, probe};
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
        interval_secs: 60,
        timeout_secs: 5,
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

    // Port 1 is almost certainly refused on loopback.
    let cfg = TcpMonitorConfig {
        name: "test-tcp-down".into(),
        host: "127.0.0.1".into(),
        port: 1,
        interval_secs: 60,
        timeout_secs: 2,
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

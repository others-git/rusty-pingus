use axum::{
    body::Body,
    http::{header, Response, StatusCode, Uri},
    response::IntoResponse,
    routing::get,
    Router,
};
use rust_embed::Embed;
use sqlx::SqlitePool;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::api;

#[derive(Embed)]
#[folder = "assets/"]
struct Assets;

async fn serve_asset(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(Body::from(content.data))
                .unwrap()
        }
        // SPA fallback: any unmatched path under /monitors/* serves monitor.html
        // so the Alpine.js app can read the name from location.pathname.
        None if path.starts_with("monitors/") => {
            match Assets::get("monitor.html") {
                Some(content) => Response::builder()
                    .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                    .body(Body::from(content.data))
                    .unwrap(),
                None => Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("Not found"))
                    .unwrap(),
            }
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not found"))
            .unwrap(),
    }
}

async fn serve_index() -> impl IntoResponse {
    match Assets::get("index.html") {
        Some(content) => Response::builder()
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(content.data))
            .unwrap(),
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("index.html not found"))
            .unwrap(),
    }
}

async fn serve_monitor_page() -> impl IntoResponse {
    match Assets::get("monitor.html") {
        Some(content) => Response::builder()
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(content.data))
            .unwrap(),
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("monitor.html not found"))
            .unwrap(),
    }
}

pub async fn serve(bind: String, pool: SqlitePool, cancel: CancellationToken) {
    let app = Router::new()
        .route("/", get(serve_index))
        .route("/monitors/{name}", get(serve_monitor_page))
        .route("/api/monitors", get(api::list_monitors))
        .route("/api/monitors/{name}/history", get(api::monitor_history))
        .route("/api/monitors/{name}/uptime", get(api::monitor_uptime))
        .route("/*path", get(serve_asset))
        .with_state(pool);

    let listener = match tokio::net::TcpListener::bind(&bind).await {
        Ok(l) => l,
        Err(e) => {
            error!(bind = %bind, error = %e, "Failed to bind web server");
            std::process::exit(1);
        }
    };

    info!("Web server listening on http://{}", bind);

    axum::serve(listener, app)
        .with_graceful_shutdown(async move { cancel.cancelled().await })
        .await
        .unwrap_or_else(|e| error!(error = %e, "Web server error"));
}

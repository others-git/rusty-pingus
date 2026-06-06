use axum::{
    body::Body,
    http::{header, HeaderMap, Response, StatusCode, Uri},
    response::IntoResponse,
    routing::{delete, get, post},
    Router,
};
use rust_embed::{Embed, EmbeddedFile};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::api::{self, AppState};

#[derive(Embed)]
#[folder = "assets/"]
struct Assets;

/// Serve an embedded asset with an ETag (content hash) and `Cache-Control:
/// no-cache`, so browsers revalidate on every load: a fast `304 Not Modified`
/// when unchanged, and fresh content the moment a file changes. In debug builds
/// rust-embed reads from disk and recomputes the hash per request (edits show
/// immediately); in release the hash is a compile-time constant.
fn embedded_response(headers: &HeaderMap, content: EmbeddedFile, content_type: &str) -> Response<Body> {
    let hash = content.metadata.sha256_hash();
    let etag = format!("\"{}\"", hex8(&hash));

    // Conditional request: if the client's cached ETag still matches, send 304.
    if let Some(inm) = headers.get(header::IF_NONE_MATCH).and_then(|v| v.to_str().ok()) {
        if inm == etag {
            return Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .header(header::ETAG, &etag)
                .header(header::CACHE_CONTROL, "no-cache")
                .body(Body::empty())
                .unwrap();
        }
    }

    Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .header(header::ETAG, &etag)
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from(content.data))
        .unwrap()
}

/// First 8 bytes of a hash as hex — short but ample to detect asset changes.
fn hex8(bytes: &[u8]) -> String {
    bytes.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

fn not_found(msg: &'static str) -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from(msg))
        .unwrap()
}

async fn serve_asset(uri: Uri, headers: HeaderMap) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            embedded_response(&headers, content, mime.as_ref())
        }
        // SPA fallback: any unmatched path under /monitors/* serves monitor.html
        None if path.starts_with("monitors/") => match Assets::get("monitor.html") {
            Some(content) => embedded_response(&headers, content, "text/html; charset=utf-8"),
            None => not_found("Not found"),
        },
        None => not_found("Not found"),
    }
}

async fn serve_index(headers: HeaderMap) -> impl IntoResponse {
    match Assets::get("index.html") {
        Some(content) => embedded_response(&headers, content, "text/html; charset=utf-8"),
        None => not_found("index.html not found"),
    }
}

async fn serve_monitor_page(headers: HeaderMap) -> impl IntoResponse {
    match Assets::get("monitor.html") {
        Some(content) => embedded_response(&headers, content, "text/html; charset=utf-8"),
        None => not_found("monitor.html not found"),
    }
}

pub async fn serve(bind: String, state: AppState, cancel: CancellationToken) {
    let app = Router::new()
        .route("/", get(serve_index))
        .route("/monitors/:name", get(serve_monitor_page))
        // Probe status API (read-only, derived from probe_results)
        .route("/api/monitors", get(api::list_monitors))
        // Live status stream (SSE) — must precede the `/*path` asset catch-all
        .route("/api/monitors/stream", get(api::monitor_stream))
        .route("/api/monitors/:name/history", get(api::monitor_history))
        .route("/api/monitors/:name/uptime", get(api::monitor_uptime))
        .route("/api/monitors/:name/series", get(api::monitor_series))
        // Traceroute per-hop data + retained extent (more specific route first)
        .route("/api/monitors/:name/traceroute/extent", get(api::monitor_traceroute_extent))
        .route("/api/monitors/:name/traceroute", get(api::monitor_traceroute))
        // Monitor config CRUD
        .route("/api/monitors/config", get(api::list_monitor_configs))
        .route("/api/monitors", post(api::add_monitor))
        .route("/api/monitors/:name/enabled", post(api::set_monitor_enabled))
        .route("/api/monitors/:name", delete(api::delete_monitor))
        // Static assets catch-all
        .route("/*path", get(serve_asset))
        .with_state(state);

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

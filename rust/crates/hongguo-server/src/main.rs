//! Minimal Rust axum slice of the hongguo-drama Web/Docker server.
//!
//! This is the L5 shell of the vertical slice (ADR-009): it keeps the Node server's security
//! contract (Bearer token, same-origin check, security headers, status codes) and calls the same
//! Rust units the desktop and Android shells will call. It is intentionally small — the full task
//! queue and media proxy are later waves — but it is buildable, runnable and testable end to end.

use axum::{
    Json, Router,
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use subtle::ConstantTimeEq;

#[derive(Clone)]
struct AppState {
    /// SHA-256 of `Bearer <token>`.
    auth_digest: [u8; 32],
    version: String,
}

#[tokio::main]
async fn main() {
    let token = std::env::var("HONGGUO_TOKEN").unwrap_or_default();
    assert!(
        token.len() >= 32 && token.len() <= 256 && !token.chars().any(char::is_whitespace),
        "必须配置 32 至 256 字符的 HONGGUO_TOKEN，不得含空白"
    );
    let mut hasher = Sha256::new();
    hasher.update(format!("Bearer {token}"));
    let auth_digest: [u8; 32] = hasher.finalize().into();
    let version = std::env::var("HONGGUO_VERSION").unwrap_or_else(|_| "7.9".into());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(3000);
    let state = AppState {
        auth_digest,
        version,
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/status", get(status))
        .route("/api/window-scale", post(window_scale))
        .route("/api/release", post(release))
        .fallback(fallback)
        .layer(middleware::from_fn(security_headers))
        .with_state(Arc::new(state));

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("bind failed");
    println!("hongguo-server listening on 0.0.0.0:{port}");
    axum::serve(listener, app).await.expect("serve failed");
}

/// The Node server sets these on every response, including errors and static files.
async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' https:; connect-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",
        ),
    );
    response
}

async fn health(State(state): State<Arc<AppState>>) -> Response {
    let _ = state;
    (StatusCode::OK, Json(json!({ "status": "ok" }))).into_response()
}

fn authorized(headers: &HeaderMap, state: &AppState) -> bool {
    let provided = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    let digest: [u8; 32] = Sha256::digest(provided.as_bytes()).into();
    bool::from(digest.ct_eq(&state.auth_digest))
}

/// Same as `index.cjs`: an Origin, when present, must match Host and use http(s).
fn origin_ok(headers: &HeaderMap) -> bool {
    let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) else {
        return true;
    };
    let Some(host) = headers.get(header::HOST).and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let Some((scheme, rest)) = origin.split_once("://") else {
        return false;
    };
    if scheme != "http" && scheme != "https" {
        return false;
    }
    let origin_host = rest.split('/').next().unwrap_or("");
    origin_host.eq_ignore_ascii_case(host)
}

fn api_error(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "success": false, "error": message }))).into_response()
}

async fn status(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if !authorized(&headers, &state) {
        return api_error(StatusCode::UNAUTHORIZED, "访问令牌无效");
    }
    if !origin_ok(&headers) {
        return api_error(StatusCode::FORBIDDEN, "只允许同源请求");
    }
    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "version": state.version,
            "downloadRoot": "/downloads",
            "concurrency": 1,
        })),
    )
        .into_response()
}

async fn window_scale(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Response {
    if !authorized(&headers, &state) {
        return api_error(StatusCode::UNAUTHORIZED, "访问令牌无效");
    }
    if !origin_ok(&headers) {
        return api_error(StatusCode::FORBIDDEN, "只允许同源请求");
    }
    let value = payload.get("value").cloned().unwrap_or(Value::Null);
    match hongguo_window_layout::port::get_window_scale(value) {
        Ok(scale) => (
            StatusCode::OK,
            Json(json!({ "success": true, "scale": scale })),
        )
            .into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, &error),
    }
}

async fn release(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Response {
    if !authorized(&headers, &state) {
        return api_error(StatusCode::UNAUTHORIZED, "访问令牌无效");
    }
    if !origin_ok(&headers) {
        return api_error(StatusCode::FORBIDDEN, "只允许同源请求");
    }
    let data = payload.get("data").cloned().unwrap_or(Value::Null);
    let current = payload
        .get("currentVersion")
        .cloned()
        .unwrap_or_else(|| json!(state.version));
    let available =
        hongguo_app_update::port::available_release(data, current).unwrap_or(Value::Null);
    (
        StatusCode::OK,
        Json(json!({ "success": true, "release": available })),
    )
        .into_response()
}

async fn fallback(method: Method) -> Response {
    if method != Method::GET && method != Method::HEAD {
        return api_error(StatusCode::METHOD_NOT_ALLOWED, "方法不允许");
    }
    api_error(StatusCode::NOT_FOUND, "页面不存在")
}

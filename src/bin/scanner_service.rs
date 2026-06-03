use axum::extract::State;
use axum::http::{header, HeaderMap, Method, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};

#[derive(Clone)]
struct AppState {
    local_allowed_origins: Arc<HashSet<String>>,
    local_token_ttl_seconds: i64,
    local_token: Arc<RwLock<Option<TokenRecord>>>,
    server_api_key: Option<String>,
}

#[derive(Clone)]
struct TokenRecord {
    value: String,
    expires_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct ConsentRequest {
    consent: bool,
}

#[derive(Serialize)]
struct TokenResponse {
    access_token: String,
    expires_at: String,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize)]
struct ServerScanResponse {
    scope: &'static str,
    report: hardware_scanner::Report,
}

#[tokio::main]
async fn main() {
    let bind_addr = env::var("SCANNER_SERVICE_BIND").unwrap_or_else(|_| "127.0.0.1:7878".to_string());
    let socket_addr: SocketAddr = bind_addr
        .parse()
        .expect("SCANNER_SERVICE_BIND must be host:port, e.g. 127.0.0.1:7878");

    let local_allowed_origins = parse_allowed_origins();
    let local_token_ttl_seconds = env::var("LOCAL_TOKEN_TTL_SECONDS")
        .ok()
        .and_then(|raw| raw.parse::<i64>().ok())
        .filter(|ttl| *ttl > 0)
        .unwrap_or(120);

    let state = AppState {
        local_allowed_origins: Arc::new(local_allowed_origins.clone()),
        local_token_ttl_seconds,
        local_token: Arc::new(RwLock::new(None)),
        server_api_key: env::var("SERVER_SCAN_API_KEY").ok(),
    };

    let cors = build_cors_layer(&local_allowed_origins);

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/local/token", post(create_local_token))
        .route("/api/local/scan", post(scan_local_machine))
        .route("/api/server/scan", post(scan_server_machine))
        .layer(cors)
        .with_state(state);

    println!("scanner service listening on http://{socket_addr}");
    let listener = tokio::net::TcpListener::bind(socket_addr)
        .await
        .expect("failed to bind scanner service listener");
    axum::serve(listener, app)
        .await
        .expect("scanner service stopped unexpectedly");
}

fn parse_allowed_origins() -> HashSet<String> {
    let raw = env::var("LOCAL_ALLOWED_ORIGINS").unwrap_or_else(|_| {
        "http://localhost:3000,http://127.0.0.1:3000,http://localhost:5173,http://127.0.0.1:5173"
            .to_string()
    });

    raw.split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .map(str::to_string)
        .collect()
}

fn build_cors_layer(origins: &HashSet<String>) -> CorsLayer {
    let origin_list = origins
        .iter()
        .filter_map(|origin| origin.parse().ok())
        .collect::<Vec<_>>();

    let allowed_origins = if origin_list.is_empty() {
        AllowOrigin::any()
    } else {
        AllowOrigin::list(origin_list)
    };

    CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods(AllowMethods::list([
            Method::GET,
            Method::POST,
            Method::OPTIONS,
        ]))
        .allow_headers(AllowHeaders::list([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::HeaderName::from_static("x-local-token"),
            header::HeaderName::from_static("x-api-key"),
        ]))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn create_local_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ConsentRequest>,
) -> impl IntoResponse {
    if !payload.consent {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "consent must be true for local hardware scanning".to_string(),
            }),
        );
    }

    if !is_origin_allowed(&headers, &state.local_allowed_origins) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "origin is not allowed for local scanning".to_string(),
            }),
        );
    }

    let token_value = generate_token();
    let expires_at = Utc::now() + chrono::TimeDelta::seconds(state.local_token_ttl_seconds);

    {
        let mut slot = state.local_token.write().await;
        *slot = Some(TokenRecord {
            value: token_value.clone(),
            expires_at,
        });
    }

    (
        StatusCode::OK,
        Json(TokenResponse {
            access_token: token_value,
            expires_at: expires_at.to_rfc3339(),
        }),
    )
}

async fn scan_local_machine(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if !is_origin_allowed(&headers, &state.local_allowed_origins) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "origin is not allowed for local scanning".to_string(),
            }),
        )
            .into_response();
    }

    if !consume_valid_local_token(&state, &headers).await {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "missing or invalid bearer token".to_string(),
            }),
        )
            .into_response();
    }

    let report = hardware_scanner::generate_report();
    (StatusCode::OK, Json(report)).into_response()
}

async fn scan_server_machine(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let Some(expected_key) = state.server_api_key.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "SERVER_SCAN_API_KEY is not configured".to_string(),
            }),
        )
            .into_response();
    };

    let provided = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    if provided != expected_key {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "invalid x-api-key".to_string(),
            }),
        )
            .into_response();
    }

    let report = hardware_scanner::generate_report();
    (
        StatusCode::OK,
        Json(ServerScanResponse {
            scope: "server",
            report,
        }),
    )
        .into_response()
}

fn is_origin_allowed(headers: &HeaderMap, allowed: &HashSet<String>) -> bool {
    let Some(origin) = headers.get(header::ORIGIN).and_then(|value| value.to_str().ok()) else {
        return false;
    };

    allowed.contains(origin)
}

async fn consume_valid_local_token(state: &AppState, headers: &HeaderMap) -> bool {
    let token = headers
        .get("x-local-token")
        .and_then(|value| value.to_str().ok())
        .or_else(|| {
            headers
                .get(header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .and_then(|auth| auth.strip_prefix("Bearer "))
        });

    let Some(token) = token else {
        return false;
    };

    let mut slot = state.local_token.write().await;
    let Some(record) = slot.as_ref() else {
        return false;
    };

    if record.expires_at <= Utc::now() {
        *slot = None;
        return false;
    }

    if record.value != token {
        return false;
    }

    *slot = None;
    true
}

fn generate_token() -> String {
    let mut rng = rand::rng();
    format!("{:032x}{:032x}", rng.random::<u128>(), rng.random::<u128>())
}

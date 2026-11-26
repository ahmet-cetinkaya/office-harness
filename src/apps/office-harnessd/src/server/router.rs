// SPDX-License-Identifier: AGPL-3.0-or-later

use std::net::SocketAddr;

use axum::Json;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{ConnectInfo, Path, Request, State};
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use serde_json::{Value, json};
use wire::{HEALTH_PATH, MAX_FRAME_BYTES, MAX_HOOK_REPORT_BYTES, PROTOCOL_VERSION, WS_PATH};

use super::connection::serve_connection;
use super::server_state::ServerState;
use super::upgrade_check::check_upgrade;

/// The route of a status-hook report; the session id is the path parameter.
const HOOK_ROUTE: &str = "/v1/hook/{session}";

const BEARER_PREFIX: &str = "Bearer ";

/// The routes, behind the peer check. The router must be served with
/// `into_make_service_with_connect_info::<SocketAddr>()`.
pub(super) fn build_router(state: ServerState) -> Router {
    Router::new()
        .route(HEALTH_PATH, get(health))
        .route(WS_PATH, get(upgrade))
        .route(HOOK_ROUTE, post(hook_report))
        .layer(middleware::from_fn(require_loopback_peer))
        .with_state(state)
}

/// FR-008: a peer that is not on the loopback interface gets a bare refusal.
async fn require_loopback_peer(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    if peer.ip().is_loopback() {
        next.run(request).await
    } else {
        StatusCode::FORBIDDEN.into_response()
    }
}

/// Reveals no office data.
async fn health() -> Json<Value> {
    Json(json!({"ok": true, "protocol": PROTOCOL_VERSION}))
}

async fn upgrade(
    State(state): State<ServerState>,
    headers: HeaderMap,
    websocket: WebSocketUpgrade,
) -> Response {
    if let Err(refusal) = check_upgrade(&headers, state.port) {
        state
            .log
            .refusal(&format!("upgrade refused: {}", refusal.reason()));
        return StatusCode::FORBIDDEN.into_response();
    }
    websocket
        .max_message_size(MAX_FRAME_BYTES)
        .max_frame_size(MAX_FRAME_BYTES)
        .on_upgrade(move |socket| serve_connection(socket, state))
}

/// A status-hook report of a `claude` session (0024). The answer is an empty
/// `200` for a report taken and one empty `404` for every refusal, so the
/// CLI reads no decision from it and a caller cannot tell a wrong secret from
/// an unknown or ended session (FR-005, FR-008).
async fn hook_report(
    State(state): State<ServerState>,
    Path(session): Path<String>,
    headers: HeaderMap,
    body: Body,
) -> StatusCode {
    if take_hook_report(&state, &session, &headers, body)
        .await
        .is_some()
    {
        return StatusCode::OK;
    }
    // The line names no session, secret, or body.
    state.log.refusal("a status hook report was refused");
    StatusCode::NOT_FOUND
}

/// The checks of a report in the order of their cost: the headers, the
/// secret, the rate of the authorized session, and last the body, so that a
/// caller without the secret makes the server read nothing.
async fn take_hook_report(
    state: &ServerState,
    session: &str,
    headers: &HeaderMap,
    body: Body,
) -> Option<()> {
    check_upgrade(headers, state.port).ok()?;
    let is_json = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/json"));
    if !is_json {
        return None;
    }
    let secret = headers
        .get(AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix(BEARER_PREFIX)?;
    let agent_id = state.hooks.authorize(session, secret)?;
    if !state.hooks.admit(session) {
        return None;
    }
    let bytes = to_bytes(body, MAX_HOOK_REPORT_BYTES).await.ok()?;
    state.hooks.deliver(agent_id, session, &bytes);
    Some(())
}

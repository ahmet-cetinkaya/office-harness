// SPDX-License-Identifier: AGPL-3.0-or-later

use std::net::SocketAddr;

use axum::Json;
use axum::Router;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde_json::{Value, json};
use wire::{HEALTH_PATH, MAX_FRAME_BYTES, PROTOCOL_VERSION, WS_PATH};

use super::connection::serve_connection;
use super::server_state::ServerState;
use super::upgrade_check::check_upgrade;

/// The routes, behind the peer check. The router must be served with
/// `into_make_service_with_connect_info::<SocketAddr>()`.
pub(super) fn build_router(state: ServerState) -> Router {
    Router::new()
        .route(HEALTH_PATH, get(health))
        .route(WS_PATH, get(upgrade))
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
            .info(&format!("upgrade refused: {}", refusal.reason()));
        return StatusCode::FORBIDDEN.into_response();
    }
    websocket
        .max_message_size(MAX_FRAME_BYTES)
        .max_frame_size(MAX_FRAME_BYTES)
        .on_upgrade(move |socket| serve_connection(socket, state))
}

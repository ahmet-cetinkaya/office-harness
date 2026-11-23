// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

/// Changes only on a breaking change; clients refuse another number.
pub const PROTOCOL_VERSION: u16 = 1;

/// The WebSocket close code for every authentication failure and for a
/// replaced token.
pub const CLOSE_UNAUTHORIZED: u16 = 4401;

/// The WebSocket close code (Try Again Later) when every handshake slot is
/// busy. It is not an authentication failure, so the client retries.
pub const CLOSE_TRY_AGAIN_LATER: u16 = 1013;

/// The largest frame the server accepts before `auth.ok`.
pub const PRE_AUTH_FRAME_LIMIT: usize = 1024;

/// The largest frame after authentication.
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;

/// How long the server waits for the client proof.
pub const AUTH_TIMEOUT: Duration = Duration::from_secs(5);

pub const WS_PATH: &str = "/v1/ws";
pub const HEALTH_PATH: &str = "/v1/health";

/// Where the status hooks of a CLI session post their reports (0024). The
/// session id follows the prefix.
pub const HOOK_PATH_PREFIX: &str = "/v1/hook/";

/// The largest status-hook report the server reads (0024/FR-009).
pub const MAX_HOOK_REPORT_BYTES: usize = 64 * 1024;

/// The file in the data folder that tells clients where the server listens.
pub const SERVER_FILE_NAME: &str = "server.json";

/// The exit code of `office-harnessd status` when no login item is
/// registered, so that the desktop core can tell "no" from a failure.
pub const LOGIN_START_NOT_REGISTERED_EXIT_CODE: u8 = 3;

/// A challenge nonce is exactly this many lowercase hex characters (32 bytes).
/// The proof joins the nonce, the port, and the role without separators, so a
/// nonce of any other length would let a port be shifted into it.
pub const NONCE_TEXT_LENGTH: usize = 64;

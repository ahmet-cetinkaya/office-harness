// SPDX-License-Identifier: AGPL-3.0-or-later

//! Wire protocol shared by the server and the desktop app: the message
//! envelope, the terminal frame codec, and the challenge-response proof.

mod auth_message;
mod envelope;
mod handshake;
mod proof;
mod protocol;

pub use auth_message::AuthMessage;
pub use envelope::{ErrorCode, Event, Request, Response};
pub use handshake::{ClientHandshake, HandshakeError};
pub use proof::{ProofKeyError, ProofRole, compute_proof, nonce_is_well_formed, proof_is_valid};
pub use protocol::{
    AUTH_TIMEOUT, CLOSE_TRY_AGAIN_LATER, CLOSE_UNAUTHORIZED, HEALTH_PATH, MAX_FRAME_BYTES,
    NONCE_TEXT_LENGTH, PRE_AUTH_FRAME_LIMIT, PROTOCOL_VERSION, SERVER_FILE_NAME, WS_PATH,
};

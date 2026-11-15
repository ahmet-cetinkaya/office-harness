// SPDX-License-Identifier: AGPL-3.0-or-later

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

use crate::protocol::NONCE_TEXT_LENGTH;

type HmacSha256 = Hmac<Sha256>;

/// Which side the proof speaks for. The role is part of the signed input, so a
/// client proof cannot be replayed as a server proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofRole {
    Client,
    Server,
}

impl ProofRole {
    fn label(self) -> &'static str {
        match self {
            Self::Client => "client",
            Self::Server => "server",
        }
    }
}

/// The MAC could not be keyed with the token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProofKeyError;

impl std::fmt::Display for ProofKeyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("the token cannot key the proof")
    }
}

impl std::error::Error for ProofKeyError {}

/// True for exactly 64 lowercase hex characters, the only nonce a proof takes.
#[must_use]
pub fn nonce_is_well_formed(nonce: &str) -> bool {
    nonce.len() == NONCE_TEXT_LENGTH
        && nonce
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

/// `hex(HMAC-SHA256(token, nonce || port || role))`. The token and the nonce
/// enter as their ASCII hex text, and the port as decimal text.
///
/// # Errors
/// `ProofKeyError` when the nonce is not well formed or the MAC rejects the
/// key.
pub fn compute_proof(
    token: &str,
    nonce: &str,
    port: u16,
    role: ProofRole,
) -> Result<String, ProofKeyError> {
    let mac = keyed_mac(token, nonce, port, role)?;
    Ok(hex::encode(mac.finalize().into_bytes()))
}

/// Checks a proof in constant time. Malformed text is simply not valid.
#[must_use]
pub fn proof_is_valid(token: &str, nonce: &str, port: u16, role: ProofRole, proof: &str) -> bool {
    let Ok(claimed) = hex::decode(proof) else {
        return false;
    };
    let Ok(mac) = keyed_mac(token, nonce, port, role) else {
        return false;
    };
    mac.verify_slice(&claimed).is_ok()
}

fn keyed_mac(
    token: &str,
    nonce: &str,
    port: u16,
    role: ProofRole,
) -> Result<HmacSha256, ProofKeyError> {
    if !nonce_is_well_formed(nonce) {
        return Err(ProofKeyError);
    }
    let mut mac = HmacSha256::new_from_slice(token.as_bytes()).map_err(|_| ProofKeyError)?;
    mac.update(nonce.as_bytes());
    mac.update(port.to_string().as_bytes());
    mac.update(role.label().as_bytes());
    Ok(mac)
}

#[cfg(test)]
#[path = "proof.tests.rs"]
mod tests;

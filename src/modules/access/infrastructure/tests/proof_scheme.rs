// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

use access_application::common::interfaces::{NonceSource, ProofScheme};
use access_domain::value_objects::{OfficeToken, RANDOM_BYTE_COUNT};
use access_infrastructure::services::{HmacProofScheme, RandomNonceSource};
use wire::{ProofRole, compute_proof};

const PORT: u16 = 4321;

#[test]
fn the_scheme_accepts_exactly_the_wire_client_proof() {
    let token = OfficeToken::from_random_bytes([3; RANDOM_BYTE_COUNT]);
    let nonce = RandomNonceSource.next_nonce().expect("nonce");
    let client_proof = compute_proof(
        token.expose_secret(),
        nonce.as_str(),
        PORT,
        ProofRole::Client,
    )
    .expect("proof");

    let scheme = HmacProofScheme;

    assert!(scheme.client_proof_is_valid(&token, &nonce, PORT, &client_proof));
    assert!(!scheme.client_proof_is_valid(&token, &nonce, PORT + 1, &client_proof));
    assert_ne!(
        scheme
            .server_proof(&token, &nonce, PORT)
            .expect("server proof"),
        client_proof
    );
}

#[test]
fn nonces_are_64_hex_characters_and_never_repeat() {
    let first = RandomNonceSource.next_nonce().expect("first");
    let second = RandomNonceSource.next_nonce().expect("second");

    assert_eq!(first.as_str().len(), 64);
    assert_ne!(first, second);
}

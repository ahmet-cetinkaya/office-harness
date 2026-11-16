// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const NONCE: &str = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
const PORT: u16 = 4242;

fn client_proof() -> String {
    compute_proof(TOKEN, NONCE, PORT, ProofRole::Client).expect("proof")
}

#[test]
fn a_proof_is_64_hex_characters_and_repeatable() {
    let proof = client_proof();

    assert_eq!(proof.len(), 64);
    assert!(proof.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert_eq!(proof, client_proof());
}

#[test]
fn the_right_proof_is_valid() {
    assert!(proof_is_valid(
        TOKEN,
        NONCE,
        PORT,
        ProofRole::Client,
        &client_proof()
    ));
}

#[test]
fn a_proof_for_another_port_is_not_valid() {
    let other_port = compute_proof(TOKEN, NONCE, PORT + 1, ProofRole::Client).expect("proof");

    assert!(!proof_is_valid(
        TOKEN,
        NONCE,
        PORT,
        ProofRole::Client,
        &other_port
    ));
}

#[test]
fn a_proof_for_another_nonce_or_token_is_not_valid() {
    let other_nonce = compute_proof(TOKEN, TOKEN, PORT, ProofRole::Client).expect("proof");
    let other_token = compute_proof(NONCE, NONCE, PORT, ProofRole::Client).expect("proof");

    assert!(!proof_is_valid(
        TOKEN,
        NONCE,
        PORT,
        ProofRole::Client,
        &other_nonce
    ));
    assert!(!proof_is_valid(
        TOKEN,
        NONCE,
        PORT,
        ProofRole::Client,
        &other_token
    ));
}

#[test]
fn a_client_proof_is_not_a_server_proof() {
    assert!(!proof_is_valid(
        TOKEN,
        NONCE,
        PORT,
        ProofRole::Server,
        &client_proof()
    ));
}

#[test]
fn malformed_proofs_are_not_valid() {
    let too_short = "0".repeat(62);
    let too_long = "0".repeat(66);
    let wrong_value = "0".repeat(64);
    for proof in [
        "",
        "zz",
        "abc",
        too_short.as_str(),
        too_long.as_str(),
        wrong_value.as_str(),
    ] {
        assert!(
            !proof_is_valid(TOKEN, NONCE, PORT, ProofRole::Client, proof),
            "{proof}"
        );
    }
}

#[test]
fn a_port_cannot_be_shifted_into_the_nonce() {
    let longer_nonce = format!("{NONCE}1");

    assert!(compute_proof(TOKEN, &longer_nonce, 5123, ProofRole::Client).is_err());
    assert!(compute_proof(TOKEN, NONCE, 15123, ProofRole::Client).is_ok());
    let proof = compute_proof(TOKEN, NONCE, 15123, ProofRole::Client).expect("proof");
    assert!(!proof_is_valid(
        TOKEN,
        &longer_nonce,
        5123,
        ProofRole::Client,
        &proof
    ));
}

#[test]
fn only_64_lowercase_hex_characters_are_a_nonce() {
    assert!(nonce_is_well_formed(NONCE));
    for nonce in ["", "abc", "A".repeat(64).as_str(), "g".repeat(64).as_str()] {
        assert!(!nonce_is_well_formed(nonce), "{nonce}");
    }
    assert!(!nonce_is_well_formed(&format!("{NONCE}1")));
}

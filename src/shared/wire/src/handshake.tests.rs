// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const NONCE: &str = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
const PORT: u16 = 4242;

fn challenge() -> String {
    AuthMessage::Challenge {
        nonce: NONCE.to_owned(),
    }
    .encode()
}

fn server_ok(token: &str, port: u16) -> String {
    let proof = compute_proof(token, NONCE, port, ProofRole::Server).expect("proof");
    AuthMessage::AuthOk { proof }.encode()
}

#[test]
fn a_server_that_proves_itself_is_confirmed() {
    let mut handshake = ClientHandshake::new(TOKEN, PORT);

    let answer = handshake.answer_challenge(&challenge()).expect("answer");

    let Some(AuthMessage::Auth { proof, .. }) = AuthMessage::decode(&answer) else {
        panic!("not an auth frame: {answer}");
    };
    assert!(proof_is_valid(
        TOKEN,
        NONCE,
        PORT,
        ProofRole::Client,
        &proof
    ));
    assert_eq!(handshake.confirm_server(&server_ok(TOKEN, PORT)), Ok(()));
}

#[test]
fn a_server_with_another_token_or_port_is_not_confirmed() {
    let mut handshake = ClientHandshake::new(TOKEN, PORT);
    handshake.answer_challenge(&challenge()).expect("answer");

    assert_eq!(
        handshake.confirm_server(&server_ok(NONCE, PORT)),
        Err(HandshakeError::ServerNotProven)
    );
    assert_eq!(
        handshake.confirm_server(&server_ok(TOKEN, PORT + 1)),
        Err(HandshakeError::ServerNotProven)
    );
}

#[test]
fn the_client_proof_is_not_accepted_as_the_server_proof() {
    let mut handshake = ClientHandshake::new(TOKEN, PORT);
    let answer = handshake.answer_challenge(&challenge()).expect("answer");
    let Some(AuthMessage::Auth { proof, .. }) = AuthMessage::decode(&answer) else {
        panic!("not an auth frame");
    };

    let echoed = AuthMessage::AuthOk { proof }.encode();

    assert_eq!(
        handshake.confirm_server(&echoed),
        Err(HandshakeError::ServerNotProven)
    );
}

#[test]
fn confirmation_needs_a_challenge_first() {
    let handshake = ClientHandshake::new(TOKEN, PORT);

    assert_eq!(
        handshake.confirm_server(&server_ok(TOKEN, PORT)),
        Err(HandshakeError::ServerNotProven)
    );
}

#[test]
fn a_first_frame_that_is_not_a_challenge_is_refused() {
    let mut handshake = ClientHandshake::new(TOKEN, PORT);

    assert_eq!(
        handshake.answer_challenge(r#"{"id":1,"ok":true}"#),
        Err(HandshakeError::NoChallenge)
    );
}

#[test]
fn a_nonce_that_is_not_64_lowercase_hex_characters_is_refused() {
    let too_long = format!("{NONCE}1");
    let too_short = NONCE[1..].to_owned();
    let not_hex = "z".repeat(64);
    let uppercase = NONCE.to_uppercase();

    for nonce in [too_long, too_short, not_hex, uppercase, String::new()] {
        let mut handshake = ClientHandshake::new(TOKEN, PORT);
        let frame = AuthMessage::Challenge {
            nonce: nonce.clone(),
        }
        .encode();

        assert_eq!(
            handshake.answer_challenge(&frame),
            Err(HandshakeError::NoChallenge),
            "{nonce}"
        );
    }
}

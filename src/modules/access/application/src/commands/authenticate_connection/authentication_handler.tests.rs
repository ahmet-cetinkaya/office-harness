// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Mutex;

use access_domain::value_objects::{Nonce, OfficeToken, RANDOM_BYTE_COUNT};

use super::*;
use crate::common::models::TokenSnapshot;

struct FixedStore(Mutex<TokenSnapshot>);

impl TokenStore for FixedStore {
    fn current(&self) -> Result<TokenSnapshot, AccessError> {
        Ok(self.0.lock().expect("lock").clone())
    }

    fn regenerate(&self) -> Result<TokenSnapshot, AccessError> {
        let mut snapshot = self.0.lock().expect("lock");
        snapshot.token = OfficeToken::from_random_bytes([9; RANDOM_BYTE_COUNT]);
        snapshot.generation += 1;
        Ok(snapshot.clone())
    }
}

struct FixedNonce;

impl NonceSource for FixedNonce {
    fn next_nonce(&self) -> Result<Nonce, AccessError> {
        Ok(Nonce::from_random_bytes([1; RANDOM_BYTE_COUNT]))
    }
}

/// A stand-in scheme: a proof is the text of its inputs.
struct TextScheme;

impl TextScheme {
    fn proof(token: &OfficeToken, nonce: &Nonce, port: u16) -> String {
        format!("{}|{}|{port}", token.expose_secret(), nonce.as_str())
    }
}

impl ProofScheme for TextScheme {
    fn client_proof_is_valid(
        &self,
        token: &OfficeToken,
        nonce: &Nonce,
        port: u16,
        proof: &str,
    ) -> bool {
        proof == Self::proof(token, nonce, port)
    }

    fn server_proof(
        &self,
        token: &OfficeToken,
        nonce: &Nonce,
        port: u16,
    ) -> Result<String, AccessError> {
        Ok(format!("server:{}", Self::proof(token, nonce, port)))
    }
}

const PORT: u16 = 5000;

fn handler() -> (AuthenticationHandler, Arc<FixedStore>) {
    let store = Arc::new(FixedStore(Mutex::new(TokenSnapshot {
        token: OfficeToken::from_random_bytes([5; RANDOM_BYTE_COUNT]),
        generation: 1,
    })));
    let handler =
        AuthenticationHandler::new(store.clone(), Arc::new(FixedNonce), Arc::new(TextScheme));
    (handler, store)
}

#[test]
fn the_right_proof_is_granted_with_the_generation_and_the_server_proof() {
    let (handler, _) = handler();
    let pending = handler.begin().expect("begin");
    let proof = TextScheme::proof(&pending.snapshot.token, pending.nonce(), PORT);

    let grant = handler.complete(&pending, PORT, &proof).expect("grant");

    assert_eq!(grant.generation, 1);
    assert_eq!(grant.server_proof, format!("server:{proof}"));
}

#[test]
fn a_wrong_or_empty_proof_is_rejected() {
    let (handler, _) = handler();
    let pending = handler.begin().expect("begin");

    for proof in ["", "wrong"] {
        let error = handler
            .complete(&pending, PORT, proof)
            .expect_err("rejected");
        assert!(matches!(error, AccessError::ProofRejected), "{error}");
    }
}

#[test]
fn a_proof_for_another_port_is_rejected() {
    let (handler, _) = handler();
    let pending = handler.begin().expect("begin");
    let proof = TextScheme::proof(&pending.snapshot.token, pending.nonce(), PORT + 1);

    assert!(handler.complete(&pending, PORT, &proof).is_err());
}

#[test]
fn a_challenge_stays_bound_to_the_token_it_was_made_with() {
    let (handler, store) = handler();
    let pending = handler.begin().expect("begin");
    store.regenerate().expect("regenerate");
    let old_proof = TextScheme::proof(&pending.snapshot.token, pending.nonce(), PORT);

    let grant = handler
        .complete(&pending, PORT, &old_proof)
        .expect("old proof still matches");

    assert_eq!(
        grant.generation, 1,
        "the caller compares it with the new generation"
    );
    let fresh = handler.begin().expect("begin");
    assert!(handler.complete(&fresh, PORT, &old_proof).is_err());
}

// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use access_application::common::errors::AccessError;
use access_application::common::interfaces::TokenStore;
use access_application::common::models::TokenSnapshot;
use access_domain::value_objects::{OfficeToken, RANDOM_BYTE_COUNT};
use serde_json::json;

use super::*;

struct CountingStore {
    regenerations: AtomicU64,
    fails: bool,
}

impl CountingStore {
    fn snapshot(generation: u64) -> TokenSnapshot {
        TokenSnapshot {
            token: OfficeToken::from_random_bytes([8; RANDOM_BYTE_COUNT]),
            generation,
        }
    }
}

impl TokenStore for CountingStore {
    fn current(&self) -> Result<TokenSnapshot, AccessError> {
        Ok(Self::snapshot(1))
    }

    fn regenerate(&self) -> Result<TokenSnapshot, AccessError> {
        if self.fails {
            return Err(AccessError::Storage("disk full".to_owned()));
        }
        let generation = self.regenerations.fetch_add(1, Ordering::SeqCst) + 2;
        Ok(Self::snapshot(generation))
    }
}

fn handlers(fails: bool) -> (AccessHandlers, Arc<CountingStore>) {
    let store = Arc::new(CountingStore {
        regenerations: AtomicU64::new(0),
        fails,
    });
    let handlers = AccessHandlers::new(RegenerateTokenHandler::new(store.clone()));
    (handlers, store)
}

fn request(id: u32, kind: &str) -> Request {
    Request {
        id,
        kind: kind.to_owned(),
        fields: serde_json::Map::new(),
    }
}

#[test]
fn regenerate_answers_ok_with_no_result_and_regenerates_once() {
    let (handlers, store) = handlers(false);

    let response = handlers.handle(&request(5, "token.regenerate"));

    assert_eq!(response, Response::success(5, json!(null)));
    assert_eq!(store.regenerations.load(Ordering::SeqCst), 1);
}

#[test]
fn a_failed_regeneration_is_an_internal_error_with_the_same_id() {
    let (handlers, _) = handlers(true);

    let response = handlers.handle(&request(6, "token.regenerate"));

    assert!(matches!(
        response,
        Response::Failure {
            id: 6,
            code: ErrorCode::Internal,
            ..
        }
    ));
}

#[test]
fn an_unknown_token_request_is_not_found() {
    let (handlers, store) = handlers(false);

    let response = handlers.handle(&request(7, "token.show"));

    assert!(matches!(
        response,
        Response::Failure {
            id: 7,
            code: ErrorCode::NotFound,
            ..
        }
    ));
    assert_eq!(store.regenerations.load(Ordering::SeqCst), 0);
}

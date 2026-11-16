// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn a_busy_server_is_shown_as_busy() {
    assert_eq!(state_after_failure(Failure::Busy), ConnectionState::Busy);
}

#[test]
fn a_rejected_proof_is_not_authorized_at_once() {
    assert_eq!(
        state_after_failure(Failure::Rejected),
        ConnectionState::NotAuthorized
    );
}

#[test]
fn a_missing_token_is_not_authorized_at_once() {
    assert_eq!(
        state_after_failure(Failure::TokenUnavailable),
        ConnectionState::NotAuthorized
    );
}

#[test]
fn a_server_that_did_not_answer_is_not_reachable() {
    assert_eq!(
        state_after_failure(Failure::NotReachable),
        ConnectionState::NotReachable
    );
}

#[test]
fn an_incompatible_server_is_a_version_mismatch() {
    let incompatibility = Incompatibility {
        server_version: "0.0.9".to_owned(),
        app_version: "0.1.0".to_owned(),
        can_restart: true,
    };

    let state = state_after_failure(Failure::Incompatible(incompatibility));

    assert_eq!(
        state,
        ConnectionState::VersionMismatch {
            server_version: "0.0.9".to_owned(),
            app_version: "0.1.0".to_owned(),
            can_restart: true,
        }
    );
}

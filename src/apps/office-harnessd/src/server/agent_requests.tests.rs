// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn every_request_that_starts_a_child_is_refused_once_the_server_stops() {
    for kind in STARTING_REQUESTS {
        assert!(refused_while_stopping(kind, true), "{kind}");
    }
}

#[test]
fn nothing_is_refused_before_the_server_stops() {
    for kind in STARTING_REQUESTS {
        assert!(!refused_while_stopping(kind, false), "{kind}");
    }
}

#[test]
fn a_stop_or_a_delete_still_runs_while_the_server_stops() {
    assert!(!refused_while_stopping("agent.stop", true));
    assert!(!refused_while_stopping("agent.delete", true));
}

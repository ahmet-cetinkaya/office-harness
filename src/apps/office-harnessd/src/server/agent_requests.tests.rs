// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use serde_json::json;

#[test]
fn every_agent_of_a_response_carries_last_line_even_without_output() {
    let mut result = json!([
        {"id": "00000000-0000-4000-8000-000000000001", "name": "Ana"},
        {"id": "00000000-0000-4000-8000-000000000002", "name": "Ben"},
    ]);
    fill_last_lines(&mut result, &|agent_id| {
        (agent_id.to_string().ends_with('1')).then(|| "Building".to_owned())
    });

    assert_eq!(result[0]["last_line"], "Building");
    assert_eq!(
        result[1].get("last_line"),
        Some(&Value::Null),
        "the key must be present with null, not omitted"
    );
}

#[test]
fn a_response_that_is_one_agent_carries_last_line_too() {
    let mut result = json!({"id": "00000000-0000-4000-8000-000000000001", "name": "Ana"});

    fill_last_lines(&mut result, &|_| None);

    assert_eq!(result.get("last_line"), Some(&Value::Null));
}

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

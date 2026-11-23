// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn state(source: StatusSource, reason: Option<&str>, ask_open: bool) -> HookState {
    HookState {
        source,
        reason: reason.map(str::to_owned),
        turn_open: true,
        ask_open,
        compacting: false,
    }
}

#[test]
fn a_session_shows_its_source_reason_ask_and_compaction() {
    let json = hook_json(Some(&state(
        StatusSource::Estimated,
        Some("hook status is off"),
        false,
    )));
    assert_eq!(
        json,
        serde_json::json!({
            "source": "estimated",
            "reason": "hook status is off",
            "ask_open": false,
            "compacting": false,
        })
    );
    assert_eq!(
        hook_json(Some(&state(StatusSource::Exact, None, true)))["ask_open"],
        true
    );
    assert_eq!(
        hook_json(Some(&state(StatusSource::Late, None, false)))["source"],
        "late"
    );
}

#[test]
fn no_session_is_null() {
    assert_eq!(hook_json(None), Value::Null);
}

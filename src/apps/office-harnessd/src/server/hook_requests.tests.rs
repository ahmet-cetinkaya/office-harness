// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn each_cli_event_name_maps_to_one_kind_and_others_to_none() {
    let pairs = [
        ("UserPromptSubmit", HookEventKind::TurnStarted),
        ("Stop", HookEventKind::TurnStopped),
        ("StopFailure", HookEventKind::TurnFailed),
        ("PermissionRequest", HookEventKind::PermissionAsked),
        ("PreCompact", HookEventKind::CompactionStarted),
        ("PostCompact", HookEventKind::CompactionEnded),
    ];
    for (name, kind) in pairs {
        assert_eq!(HookEventKind::from_cli_event(name), Some(kind));
    }
    assert_eq!(HookEventKind::from_cli_event("PreToolUse"), None);
    assert_eq!(HookEventKind::from_cli_event(""), None);
}

#[test]
fn a_body_keeps_only_its_kind_and_the_helper_flag() {
    let body =
        br#"{"hook_event_name":"Stop","prompt":"secret words","last_assistant_message":"x"}"#;
    assert_eq!(
        read_report_body(body),
        Some((HookEventKind::TurnStopped, false))
    );
    let helper = br#"{"hook_event_name":"PermissionRequest","agent_id":"a1"}"#;
    assert_eq!(
        read_report_body(helper),
        Some((HookEventKind::PermissionAsked, true))
    );
    let null_agent = br#"{"hook_event_name":"Stop","agent_id":null}"#;
    assert_eq!(
        read_report_body(null_agent),
        Some((HookEventKind::TurnStopped, false))
    );
}

#[test]
fn a_body_that_is_not_a_known_report_is_no_report() {
    assert_eq!(read_report_body(b"not json"), None);
    assert_eq!(read_report_body(b"{}"), None);
    assert_eq!(read_report_body(br#"{"hook_event_name":"Other"}"#), None);
    assert_eq!(read_report_body(br#"{"hook_event_name":7}"#), None);
    assert_eq!(read_report_body(b""), None);
}

#[test]
fn the_limiter_admits_fifty_a_second_per_session_and_starts_again() {
    let mut limiter = RateLimiter::default();
    let start = Instant::now();
    for _ in 0..REPORTS_PER_SECOND {
        assert!(limiter.admit("s1", start));
    }
    assert!(!limiter.admit("s1", start));
    assert!(
        limiter.admit("s2", start),
        "another session has its own window"
    );
    assert!(limiter.admit("s1", start + RATE_WINDOW));
}

#[test]
fn the_limiter_forgets_a_session_that_has_been_quiet() {
    let mut limiter = RateLimiter::default();
    let start = Instant::now();
    assert!(limiter.admit("s1", start));
    assert!(limiter.admit("s2", start + IDLE_WINDOW_LIFETIME));
    assert_eq!(limiter.windows.len(), 1);
}

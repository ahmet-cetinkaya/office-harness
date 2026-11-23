// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn agent() -> AgentId {
    AgentId::from_u128(1)
}

fn run() -> RunId {
    RunId(1)
}

fn sessions() -> HookSessions {
    let sessions = HookSessions::new(WaitTracker::default());
    sessions.session_started(agent(), Some(run()), None);
    sessions
}

fn report(kind: HookEventKind) -> HookReport {
    HookReport {
        agent_id: agent(),
        session: "s".to_owned(),
        kind,
        from_helper: false,
    }
}

fn apply(sessions: &HookSessions, kind: HookEventKind) -> ReportOutcome {
    sessions
        .apply_report(&report(kind), run())
        .expect("outcome")
}

#[test]
fn a_session_is_estimated_until_its_first_report() {
    let sessions = sessions();
    assert_eq!(
        sessions.state(agent()).expect("state").source,
        StatusSource::Estimated
    );
    assert_eq!(
        sessions.judge_terminal_event(agent(), TerminalEvent::QuietPeriod),
        TerminalVerdict::Pass
    );
    apply(&sessions, HookEventKind::TurnStarted);
    assert_eq!(
        sessions.state(agent()).expect("state").source,
        StatusSource::Exact
    );
}

#[test]
fn a_turn_works_from_its_start_to_its_stop() {
    let sessions = sessions();
    let started = apply(&sessions, HookEventKind::TurnStarted);
    assert_eq!(started.status_event, StatusEvent::Output);
    assert_eq!(started.detail, None);
    let stopped = apply(&sessions, HookEventKind::TurnStopped);
    assert_eq!(stopped.status_event, StatusEvent::QuietPeriodElapsed);
    assert_eq!(stopped.detail, Some(DETAIL_TURN_STOPPED));
}

#[test]
fn a_failed_turn_waits_with_its_own_word() {
    let sessions = sessions();
    apply(&sessions, HookEventKind::TurnStarted);
    let failed = apply(&sessions, HookEventKind::TurnFailed);
    assert_eq!(failed.status_event, StatusEvent::QuietPeriodElapsed);
    assert_eq!(failed.detail, Some(DETAIL_TURN_FAILED));
}

#[test]
fn a_permission_ask_waits_inside_an_open_turn_and_ends_with_the_next_report() {
    let sessions = sessions();
    apply(&sessions, HookEventKind::TurnStarted);
    let asked = apply(&sessions, HookEventKind::PermissionAsked);
    assert_eq!(asked.status_event, StatusEvent::QuietPeriodElapsed);
    assert_eq!(asked.detail, Some(DETAIL_PERMISSION_ASKED));
    assert!(asked.ask_opened);
    assert!(sessions.ask_open(agent()));
    assert!(sessions.blocks_typing(agent()));
    let compacted = apply(&sessions, HookEventKind::CompactionStarted);
    assert_eq!(compacted.status_event, StatusEvent::Output);
    assert!(!sessions.ask_open(agent()));
}

#[test]
fn a_person_input_ends_an_open_ask_and_the_turn_goes_on() {
    let sessions = sessions();
    apply(&sessions, HookEventKind::TurnStarted);
    apply(&sessions, HookEventKind::PermissionAsked);
    assert_eq!(sessions.input_ends_ask(agent()), Some(StatusEvent::Output));
    assert!(!sessions.ask_open(agent()));
    assert_eq!(
        sessions.input_ends_ask(agent()),
        None,
        "only an open ask ends"
    );
}

#[test]
fn a_compaction_keeps_the_agent_working_and_a_stop_inside_it_does_not_end_the_work() {
    let sessions = sessions();
    apply(&sessions, HookEventKind::TurnStarted);
    apply(&sessions, HookEventKind::CompactionStarted);
    assert!(sessions.blocks_typing(agent()));
    let stopped = apply(&sessions, HookEventKind::TurnStopped);
    assert_eq!(
        stopped.status_event,
        StatusEvent::Output,
        "compaction still runs"
    );
    let ended = apply(&sessions, HookEventKind::CompactionEnded);
    assert_eq!(ended.status_event, StatusEvent::QuietPeriodElapsed);
    assert_eq!(ended.detail, Some(DETAIL_TURN_STOPPED));
    assert!(!sessions.blocks_typing(agent()));
}

#[test]
fn a_manual_compaction_with_no_turn_works_then_waits() {
    let sessions = sessions();
    assert_eq!(
        apply(&sessions, HookEventKind::CompactionStarted).status_event,
        StatusEvent::Output
    );
    assert_eq!(
        apply(&sessions, HookEventKind::CompactionEnded).status_event,
        StatusEvent::QuietPeriodElapsed
    );
}

#[test]
fn output_and_the_quiet_period_do_not_decide_in_an_exact_session() {
    let sessions = sessions();
    apply(&sessions, HookEventKind::TurnStarted);
    apply(&sessions, HookEventKind::TurnStopped);
    for event in [TerminalEvent::OutputResumed, TerminalEvent::QuietPeriod] {
        assert_eq!(
            sessions.judge_terminal_event(agent(), event),
            TerminalVerdict::Drop
        );
    }
    for event in [TerminalEvent::FirstOutput, TerminalEvent::Bell] {
        assert_eq!(
            sessions.judge_terminal_event(agent(), event),
            TerminalVerdict::Pass
        );
    }
}

#[test]
fn a_silent_open_turn_turns_the_session_late_and_the_next_report_makes_it_exact() {
    let sessions = sessions();
    apply(&sessions, HookEventKind::TurnStarted);
    assert_eq!(
        sessions.judge_terminal_event(agent(), TerminalEvent::QuietPeriod),
        TerminalVerdict::Pass
    );
    let state = sessions.state(agent()).expect("state");
    assert_eq!(state.source, StatusSource::Late);
    assert_eq!(state.reason.as_deref(), Some(STALLED_TURN_REASON));
    assert_eq!(
        sessions.judge_terminal_event(agent(), TerminalEvent::OutputResumed),
        TerminalVerdict::Pass,
        "a late session follows the quiet period"
    );
    apply(&sessions, HookEventKind::TurnStarted);
    assert_eq!(
        sessions.state(agent()).expect("state").source,
        StatusSource::Exact
    );
}

#[test]
fn a_silent_open_ask_is_not_a_stalled_turn_but_a_silent_compaction_is() {
    let sessions = sessions();
    apply(&sessions, HookEventKind::TurnStarted);
    apply(&sessions, HookEventKind::PermissionAsked);
    assert_eq!(
        sessions.judge_terminal_event(agent(), TerminalEvent::QuietPeriod),
        TerminalVerdict::Drop
    );
    apply(&sessions, HookEventKind::CompactionStarted);
    assert_eq!(
        sessions.judge_terminal_event(agent(), TerminalEvent::QuietPeriod),
        TerminalVerdict::Pass,
        "the CLI draws a moving indicator while it compacts"
    );
}

#[test]
fn reports_of_helpers_earlier_runs_and_unknown_sessions_change_nothing() {
    let sessions = sessions();
    let helper = HookReport {
        from_helper: true,
        ..report(HookEventKind::TurnStarted)
    };
    assert_eq!(sessions.apply_report(&helper, run()), None);
    assert_eq!(
        sessions.apply_report(&report(HookEventKind::TurnStarted), RunId(2)),
        None
    );
    let other = HookReport {
        agent_id: AgentId::from_u128(9),
        ..report(HookEventKind::TurnStarted)
    };
    assert_eq!(sessions.apply_report(&other, run()), None);
    assert_eq!(
        sessions.state(agent()).expect("state").source,
        StatusSource::Estimated
    );
}

#[test]
fn an_ended_session_has_no_state() {
    let sessions = sessions();
    sessions.session_ended(agent());
    assert_eq!(sessions.state(agent()), None);
    assert!(!sessions.blocks_typing(agent()));
}

// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::Timestamp;
use notification_domain::{AttentionEvent, AttentionKind, ItemReport, record_event, record_report};
use terminal_application::common::errors::TerminalRuntimeError;
use terminal_application::common::interfaces::OutputSubscriber;
use terminal_application::common::models::{
    Attachment, RunId, SpawnTerminalRequest, SubscriptionId,
};
use terminal_domain::value_objects::{OutputKey, ScreenView, TerminalSize};

use super::*;

const NOW: Timestamp = Timestamp::from_unix_millis(1_700_000_000_000);

/// A runtime that shows one fixed screen, or none.
struct FixedScreen(Option<ScreenView>);

impl TerminalRuntime for FixedScreen {
    fn spawn(&self, _request: &SpawnTerminalRequest) -> Result<(), TerminalRuntimeError> {
        Ok(())
    }

    fn attach(
        &self,
        agent_id: AgentId,
        _saved_output: Option<&OutputKey>,
        _size: TerminalSize,
        _subscriber: Arc<dyn OutputSubscriber>,
    ) -> Result<Attachment, TerminalRuntimeError> {
        Err(TerminalRuntimeError::NotRunning(agent_id))
    }

    fn detach(&self, _agent_id: AgentId, _subscription: SubscriptionId) {}

    fn write_input(&self, agent_id: AgentId, _bytes: &[u8]) -> Result<(), TerminalRuntimeError> {
        Err(TerminalRuntimeError::NotRunning(agent_id))
    }

    fn paste(&self, agent_id: AgentId, _text: &str) -> Result<(), TerminalRuntimeError> {
        Err(TerminalRuntimeError::NotRunning(agent_id))
    }

    fn resize(
        &self,
        _agent_id: AgentId,
        _size: TerminalSize,
    ) -> Result<bool, TerminalRuntimeError> {
        Ok(false)
    }

    fn stop(&self, _agent_id: AgentId, _grace: Duration) -> Result<(), TerminalRuntimeError> {
        Ok(())
    }

    fn is_running(&self, _agent_id: AgentId) -> bool {
        true
    }

    fn current_run(&self, _agent_id: AgentId) -> Option<RunId> {
        None
    }

    fn screen(&self, _agent_id: AgentId) -> Option<ScreenView> {
        self.0.clone()
    }

    fn quiet_for(&self, _agent_id: AgentId) -> Option<Duration> {
        None
    }
}

fn screen_of(rows: &[&str]) -> FixedScreen {
    FixedScreen(Some(ScreenView {
        rows: rows.iter().map(|row| (*row).to_owned()).collect(),
        cursor_row: rows.len().saturating_sub(1),
        cursor_column: 0,
    }))
}

fn item_of(kind: AttentionKind) -> NeedsYouItem {
    let event = AttentionEvent {
        kind,
        detail: None,
        at: NOW,
    };
    record_event(None, agent(), event, None)
}

fn verdict(items: &[NeedsYouItem], runtime: &FixedScreen) -> Result<(), String> {
    item_allows_sleep(items, runtime, agent())
}

fn agent() -> AgentId {
    AgentId::from_u128(7)
}

#[test]
fn no_agent_sleeps_before_the_gate_is_connected() {
    let gate = IdleSleepGate::default();
    gate.set_rule(Some(IdleRule {
        idle_time: Duration::ZERO,
        counted_from: Instant::now(),
    }));
    assert_eq!(
        gate.blocking_reason(agent()).as_deref(),
        Some(NOT_CONNECTED)
    );
    assert_eq!(gate.last_prompted_wait(agent()), None);
}

#[test]
fn an_agent_is_going_to_sleep_from_begin_to_end() {
    let gate = IdleSleepGate::default();
    let sleeping = gate.sleeping();
    assert!(!sleeping.contains(agent()));
    gate.begin(agent());
    assert!(sleeping.contains(agent()));
    gate.end(agent());
    assert!(!sleeping.contains(agent()));
}

#[test]
fn an_agent_without_an_item_may_sleep_whatever_its_screen_shows() {
    assert_eq!(verdict(&[], &FixedScreen(None)), Ok(()));
}

#[test]
fn a_plain_waiting_item_on_a_clear_screen_does_not_keep_the_agent_awake() {
    let items = [item_of(AttentionKind::WaitingForInput)];
    assert_eq!(verdict(&items, &screen_of(&["ready", "> "])), Ok(()));
}

#[test]
fn an_item_that_recorded_more_than_waits_keeps_the_agent_awake() {
    let clear = screen_of(&["ready", "> "]);
    for kind in [
        AttentionKind::Bell,
        AttentionKind::AutoHold,
        AttentionKind::ResumeHold,
    ] {
        assert_eq!(
            verdict(&[item_of(kind)], &clear),
            Err(WAITS_IN_NEEDS_YOU.to_owned()),
            "{kind:?}"
        );
    }
    let report = ItemReport {
        kind: "question".to_owned(),
        text: "pick one".to_owned(),
        at: NOW,
    };
    let reported = record_report(None, agent(), report, None);
    assert_eq!(
        verdict(&[reported], &clear),
        Err(WAITS_IN_NEEDS_YOU.to_owned())
    );
}

#[test]
fn a_waiting_item_keeps_the_agent_awake_while_the_screen_shows_an_open_question() {
    let items = [item_of(AttentionKind::WaitingForInput)];
    let yes_no = screen_of(&["Overwrite the file? (y/n)"]);
    let choice_list = screen_of(&["Run this command?", "❯ 1. Yes", "  2. No"]);
    let permission = screen_of(&["Do you want to proceed?"]);
    for runtime in [yes_no, choice_list, permission] {
        assert_eq!(
            verdict(&items, &runtime),
            Err(QUESTION_SHOWS.to_owned()),
            "{:?}",
            runtime.0
        );
    }
}

#[test]
fn a_waiting_item_keeps_the_agent_awake_while_its_screen_cannot_be_read() {
    let items = [item_of(AttentionKind::WaitingForInput)];
    // SC-006 counts 10 sleep checks.
    for _ in 0..10 {
        assert_eq!(
            verdict(&items, &FixedScreen(None)),
            Err(SCREEN_UNREADABLE.to_owned())
        );
    }
}

#[test]
fn the_idle_end_of_a_temp_stops_for_a_plain_waiting_item_too() {
    let items = [item_of(AttentionKind::WaitingForInput)];
    assert_eq!(
        item_blocks_idle_end(&items, agent()),
        Err(WAITS_IN_NEEDS_YOU.to_owned())
    );
    assert_eq!(item_blocks_idle_end(&[], agent()), Ok(()));
}

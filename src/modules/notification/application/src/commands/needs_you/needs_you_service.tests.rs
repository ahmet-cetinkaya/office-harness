// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use kernel::{AgentId, Timestamp};
use notification_domain::{AttentionEvent, AttentionKind, ItemReport, NotificationId};

use super::*;
use crate::common::errors::NotificationApplicationError;
use crate::test_support::{MemoryNeedsYou, NoNames};

fn service() -> NeedsYouService {
    NeedsYouService::new(Arc::new(MemoryNeedsYou::default()), Arc::new(NoNames))
}

fn event(kind: AttentionKind, at: i64) -> AttentionEvent {
    AttentionEvent {
        kind,
        detail: None,
        at: Timestamp::from_unix_millis(at),
    }
}

#[test]
fn a_dismissed_agent_starts_a_new_item_at_the_new_time() {
    let needs_you = service();
    let agent = AgentId::generate();
    needs_you
        .record(agent, event(AttentionKind::Bell, 10), None)
        .expect("record");
    needs_you.dismiss(agent).expect("dismiss");

    let item = needs_you
        .record(agent, event(AttentionKind::Bell, 90), None)
        .expect("record again");

    assert_eq!(item.first_event_at, Timestamp::from_unix_millis(90));
}

#[test]
fn dismissing_an_agent_without_an_item_is_not_found() {
    let outcome = service().dismiss(AgentId::generate());

    assert!(matches!(
        outcome,
        Err(NotificationApplicationError::NotFound(_))
    ));
}

#[test]
fn reading_the_origin_notification_clears_the_item() {
    let needs_you = service();
    let origin = NotificationId::generate();
    needs_you
        .record(
            AgentId::generate(),
            event(AttentionKind::Exited, 10),
            Some(origin),
        )
        .expect("record");

    let cleared = needs_you.clear_on_read(origin).expect("clear");

    assert!(cleared);
    assert_eq!(needs_you.list().expect("list"), []);
}

#[test]
fn a_hold_release_clears_an_auto_hold_or_resume_hold_item_only() {
    let needs_you = service();
    let held = AgentId::generate();
    let resume_held = AgentId::generate();
    let waiting = AgentId::generate();
    needs_you
        .record(held, event(AttentionKind::AutoHold, 10), None)
        .expect("record");
    needs_you
        .record(resume_held, event(AttentionKind::ResumeHold, 15), None)
        .expect("record");
    needs_you
        .record(waiting, event(AttentionKind::WaitingForInput, 20), None)
        .expect("record");

    let held_cleared = needs_you
        .clear(held, NeedsYouClearReason::HoldReleased)
        .expect("clear");
    let resume_held_cleared = needs_you
        .clear(resume_held, NeedsYouClearReason::HoldReleased)
        .expect("clear");
    let waiting_cleared = needs_you
        .clear(waiting, NeedsYouClearReason::HoldReleased)
        .expect("clear");

    assert!(held_cleared);
    assert!(resume_held_cleared);
    assert!(!waiting_cleared);
}

#[test]
fn a_status_change_clears_the_item() {
    let needs_you = service();
    let agent = AgentId::generate();
    needs_you
        .record(agent, event(AttentionKind::Failed, 10), None)
        .expect("record");

    let cleared = needs_you
        .clear(agent, NeedsYouClearReason::Stopped)
        .expect("clear");

    assert!(cleared);
}

#[test]
fn forgetting_missing_agents_keeps_the_existing_ones() {
    let needs_you = service();
    let kept = AgentId::generate();
    let deleted = AgentId::generate();
    for agent in [kept, deleted] {
        needs_you
            .record(agent, event(AttentionKind::Bell, 10), None)
            .expect("record");
    }

    let removed = needs_you.forget_missing(&[kept]).expect("forget");

    let agents: Vec<AgentId> = needs_you
        .list()
        .expect("list")
        .iter()
        .map(|item| item.agent_id)
        .collect();
    assert!(removed);
    assert_eq!(agents, [kept]);
}

fn report(at: i64) -> ItemReport {
    ItemReport {
        kind: "question".to_owned(),
        text: "which branch?".to_owned(),
        at: Timestamp::from_unix_millis(at),
    }
}

#[test]
fn a_report_survives_a_resume_and_a_hold_release_but_not_a_stop() {
    let needs_you = service();
    let agent = AgentId::generate();
    needs_you
        .record(agent, event(AttentionKind::AutoHold, 5), None)
        .expect("held");
    needs_you
        .record_report(agent, report(10), None)
        .expect("report");
    needs_you
        .record(agent, event(AttentionKind::ResumeHold, 20), None)
        .expect("resume hold");

    let resumed = needs_you
        .clear(agent, NeedsYouClearReason::Resumed)
        .expect("resumed");
    let released = needs_you
        .clear(agent, NeedsYouClearReason::HoldReleased)
        .expect("released");
    let kept = needs_you.list().expect("list");
    let stopped = needs_you
        .clear(agent, NeedsYouClearReason::Stopped)
        .expect("stopped");

    assert!(!resumed);
    assert!(!released);
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].report, Some(report(10)));
    assert_eq!(kept[0].first_event_at, Timestamp::from_unix_millis(5));
    assert!(stopped);
}

#[test]
fn an_item_without_a_report_still_leaves_on_a_resume() {
    let needs_you = service();
    let agent = AgentId::generate();
    needs_you
        .record(agent, event(AttentionKind::WaitingForInput, 10), None)
        .expect("record");

    let cleared = needs_you
        .clear(agent, NeedsYouClearReason::Resumed)
        .expect("clear");

    assert!(cleared);
}

#[test]
fn opening_the_terminal_or_dismissing_clears_a_report_item() {
    let needs_you = service();
    let opened = AgentId::generate();
    let dismissed = AgentId::generate();
    for agent in [opened, dismissed] {
        needs_you
            .record_report(agent, report(10), None)
            .expect("report");
    }

    let cleared = needs_you
        .clear(opened, NeedsYouClearReason::TerminalOpened)
        .expect("opened");
    let left = needs_you.dismiss(dismissed).expect("dismiss");

    assert!(cleared);
    assert_eq!(left, Vec::<notification_domain::NeedsYouItem>::new());
}

// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use kernel::{AgentId, Timestamp};

use super::*;
use crate::value_objects::{AttentionEvent, AttentionKind, NotificationId};

fn event(kind: AttentionKind, at: i64) -> AttentionEvent {
    AttentionEvent {
        kind,
        detail: None,
        at: Timestamp::from_unix_millis(at),
    }
}

fn item(agent_id: AgentId, kind: AttentionKind, first_event_at: i64) -> NeedsYouItem {
    record_event(None, agent_id, event(kind, first_event_at), None)
}

fn names(pairs: &[(AgentId, &str)]) -> impl Fn(AgentId) -> String {
    let by_agent: HashMap<AgentId, String> = pairs
        .iter()
        .map(|(agent_id, name)| (*agent_id, (*name).to_owned()))
        .collect();
    move |agent_id| by_agent.get(&agent_id).cloned().unwrap_or_default()
}

fn agents_of(items: &[NeedsYouItem]) -> Vec<AgentId> {
    items.iter().map(|item| item.agent_id).collect()
}

#[test]
fn failed_comes_before_waiting_and_waiting_before_exited() {
    let exited = AgentId::generate();
    let waiting = AgentId::generate();
    let failed = AgentId::generate();
    // The exited agent waited longest, so only its group can put it last.
    let items = vec![
        item(exited, AttentionKind::Exited, 1),
        item(waiting, AttentionKind::WaitingForInput, 2),
        item(failed, AttentionKind::Failed, 3),
    ];

    let ordered = order(items, names(&[]));

    assert_eq!(agents_of(&ordered), [failed, waiting, exited]);
}

#[test]
fn waiting_bell_and_auto_hold_share_one_group_ordered_by_wait() {
    let held = AgentId::generate();
    let bell = AgentId::generate();
    let waiting = AgentId::generate();
    let items = vec![
        item(bell, AttentionKind::Bell, 20),
        item(waiting, AttentionKind::WaitingForInput, 30),
        item(held, AttentionKind::AutoHold, 10),
    ];

    let ordered = order(items, names(&[]));

    assert_eq!(agents_of(&ordered), [held, bell, waiting]);
}

#[test]
fn a_resume_hold_waits_in_the_same_group_as_an_auto_hold() {
    let resume_held = AgentId::generate();
    let failed = AgentId::generate();
    let exited = AgentId::generate();
    let items = vec![
        item(exited, AttentionKind::Exited, 1),
        item(resume_held, AttentionKind::ResumeHold, 20),
        item(failed, AttentionKind::Failed, 30),
    ];

    let ordered = order(items, names(&[]));

    assert_eq!(agents_of(&ordered), [failed, resume_held, exited]);
}

#[test]
fn the_longest_wait_comes_first_inside_a_group() {
    let newer = AgentId::generate();
    let older = AgentId::generate();
    let items = vec![
        item(newer, AttentionKind::Failed, 50),
        item(older, AttentionKind::Failed, 5),
    ];

    let ordered = order(items, names(&[]));

    assert_eq!(agents_of(&ordered), [older, newer]);
}

#[test]
fn equal_waits_are_ordered_by_agent_name() {
    let ana = AgentId::generate();
    let raj = AgentId::generate();
    let items = vec![
        item(raj, AttentionKind::Bell, 10),
        item(ana, AttentionKind::Bell, 10),
    ];

    let ordered = order(items, names(&[(ana, "Ana"), (raj, "Raj")]));

    assert_eq!(agents_of(&ordered), [ana, raj]);
}

#[test]
fn a_second_event_keeps_the_wait_time_and_changes_the_latest() {
    let agent = AgentId::generate();
    let first = item(agent, AttentionKind::Bell, 10);

    let second = record_event(
        Some(first.clone()),
        agent,
        event(AttentionKind::WaitingForInput, 40),
        Some(NotificationId::generate()),
    );

    assert_eq!(second.first_event_at, first.first_event_at);
    assert_eq!(second.latest, event(AttentionKind::WaitingForInput, 40));
    assert_eq!(second.origin, first.origin);
}

#[test]
fn a_waiting_agent_that_fails_moves_to_the_failed_group_and_keeps_its_wait() {
    let waiting_long = AgentId::generate();
    let failing = AgentId::generate();
    let waiting_since_early = item(waiting_long, AttentionKind::WaitingForInput, 5);
    let waiting_since_late = item(failing, AttentionKind::WaitingForInput, 50);

    let failed = record_event(
        Some(waiting_since_late),
        failing,
        event(AttentionKind::Failed, 90),
        None,
    );
    let ordered = order(vec![waiting_since_early, failed.clone()], names(&[]));

    assert_eq!(failed.first_event_at, Timestamp::from_unix_millis(50));
    assert_eq!(agents_of(&ordered), [failing, waiting_long]);
}

#[test]
fn an_agent_without_an_item_gets_a_new_one_with_its_origin_at_the_event_time() {
    let agent = AgentId::generate();
    let origin = NotificationId::generate();

    let created = record_event(None, agent, event(AttentionKind::Exited, 70), Some(origin));

    assert_eq!(created.first_event_at, Timestamp::from_unix_millis(70));
    assert_eq!(created.origin, Some(origin));
}

fn report(kind: &str, text: &str, at: i64) -> ItemReport {
    ItemReport {
        kind: kind.to_owned(),
        text: text.to_owned(),
        at: Timestamp::from_unix_millis(at),
    }
}

#[test]
fn a_report_starts_an_item_in_the_waiting_group() {
    let asking = AgentId::generate();
    let exited = AgentId::generate();

    let reported = record_report(None, asking, report("question", "which branch?", 30), None);
    let items = order(
        vec![item(exited, AttentionKind::Exited, 10), reported.clone()],
        names(&[]),
    );

    assert_eq!(agents_of(&items), vec![asking, exited]);
    assert_eq!(reported.latest.kind, AttentionKind::Report);
    assert_eq!(reported.latest.detail.as_deref(), Some("question"));
    assert_eq!(reported.first_event_at, Timestamp::from_unix_millis(30));
}

#[test]
fn a_later_event_keeps_the_report_and_a_second_report_replaces_it() {
    let agent = AgentId::generate();
    let first = record_report(None, agent, report("question", "first", 10), None);

    let waited = record_event(
        Some(first),
        agent,
        event(AttentionKind::WaitingForInput, 20),
        None,
    );
    let second = record_report(
        Some(waited.clone()),
        agent,
        report("done", "second", 30),
        None,
    );

    assert_eq!(waited.latest.kind, AttentionKind::WaitingForInput);
    assert_eq!(waited.report, Some(report("question", "first", 10)));
    assert_eq!(second.report, Some(report("done", "second", 30)));
    assert_eq!(second.first_event_at, Timestamp::from_unix_millis(10));
}

#[test]
fn a_report_on_a_waiting_item_keeps_its_wait_time_and_origin() {
    let agent = AgentId::generate();
    let origin = NotificationId::generate();
    let waiting = record_event(
        None,
        agent,
        event(AttentionKind::WaitingForInput, 5),
        Some(origin),
    );

    let reported = record_report(Some(waiting), agent, report("blocked", "no key", 40), None);

    assert_eq!(reported.first_event_at, Timestamp::from_unix_millis(5));
    assert_eq!(reported.origin, Some(origin));
}

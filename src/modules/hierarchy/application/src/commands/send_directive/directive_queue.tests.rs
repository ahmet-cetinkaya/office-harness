// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

const START_MILLIS: i64 = 1_700_000_000_000;

fn at(offset: Duration) -> Timestamp {
    let millis = i64::try_from(offset.as_millis()).expect("small offset");
    Timestamp::from_unix_millis(START_MILLIS + millis)
}

fn target() -> AgentId {
    AgentId::from_u128(7)
}

fn waiting(text: &str, accepted_at: Timestamp) -> WaitingDirective {
    WaitingDirective {
        sender: Sender::Agent(AgentId::from_u128(1)),
        target: target(),
        text: text.to_owned(),
        accepted_at,
        origin: Origin::Directive,
        from_temp: false,
    }
}

fn report_from(sender: u128, text: &str, accepted_at: Timestamp) -> WaitingDirective {
    WaitingDirective {
        sender: Sender::Agent(AgentId::from_u128(sender)),
        target: target(),
        text: text.to_owned(),
        accepted_at,
        origin: Origin::Report {
            text: text.to_owned(),
        },
        from_temp: false,
    }
}

#[test]
fn an_empty_target_is_claimed_once_until_the_claim_ends() {
    let queue = DirectiveQueue::default();
    assert!(queue.claim_empty(target(), at(Duration::ZERO)));
    assert!(!queue.claim_empty(target(), at(Duration::ZERO)));
    queue.release(target());
    assert!(queue.claim_empty(target(), at(Duration::ZERO)));
}

#[test]
fn a_target_with_a_waiting_directive_is_not_claimed_empty() {
    let queue = DirectiveQueue::default();
    queue.push(waiting("first", at(Duration::ZERO)));
    assert!(!queue.claim_empty(target(), at(Duration::ZERO)));
}

#[test]
fn places_count_from_one_in_the_order_of_arrival() {
    let queue = DirectiveQueue::default();
    assert_eq!(queue.push(waiting("first", at(Duration::ZERO))), Some(1));
    assert_eq!(queue.push(waiting("second", at(Duration::ZERO))), Some(2));
    assert_eq!(queue.waiting_count(target()), 2);
    let claim = queue.claim_head(target(), at(Duration::ZERO));
    assert_eq!(claim.head.map(|head| head.text).as_deref(), Some("first"));
}

#[test]
fn a_head_past_the_wait_limit_expires_and_the_next_is_claimed() {
    let queue = DirectiveQueue::default();
    queue.push(waiting("old", at(Duration::ZERO)));
    queue.push(waiting("new", at(WAIT_LIMIT)));
    let claim = queue.claim_head(target(), at(WAIT_LIMIT));
    assert_eq!(claim.expired.len(), 1);
    assert_eq!(claim.expired[0].text, "old");
    assert_eq!(claim.head.map(|head| head.text).as_deref(), Some("new"));
}

#[test]
fn the_next_delivery_waits_the_settle_gap() {
    let queue = DirectiveQueue::default();
    assert!(queue.claim_empty(target(), at(Duration::ZERO)));
    queue.finish(target(), at(Duration::ZERO));
    queue.push(waiting("next", at(Duration::ZERO)));
    let early = SETTLE_AFTER_TYPING
        .checked_sub(Duration::from_millis(1))
        .expect("the gap is longer than 1 ms");
    assert!(queue.claim_head(target(), at(early)).head.is_none());
    assert!(
        queue
            .claim_head(target(), at(SETTLE_AFTER_TYPING))
            .head
            .is_some()
    );
}

#[test]
fn a_claimed_head_is_not_claimed_twice() {
    let queue = DirectiveQueue::default();
    queue.push(waiting("first", at(Duration::ZERO)));
    assert!(
        queue
            .claim_head(target(), at(Duration::ZERO))
            .head
            .is_some()
    );
    assert!(
        queue
            .claim_head(target(), at(Duration::ZERO))
            .head
            .is_none()
    );
}

#[test]
fn draining_empties_the_queues() {
    let queue = DirectiveQueue::default();
    queue.push(waiting("first", at(Duration::ZERO)));
    queue.push(waiting("second", at(Duration::ZERO)));
    assert_eq!(queue.targets_with_waiting(), [target()]);
    assert_eq!(queue.drain(target()).len(), 2);
    queue.push(waiting("third", at(Duration::ZERO)));
    assert_eq!(queue.close().len(), 1);
    assert_eq!(queue.waiting_count(target()), 0);
    assert!(!queue.wait_for_work(Duration::from_millis(1)));
}

#[test]
fn a_closed_queue_takes_in_and_hands_out_nothing() {
    let queue = DirectiveQueue::default();
    queue.push(waiting("claimed", at(Duration::ZERO)));
    let claimed = queue
        .claim_head(target(), at(Duration::ZERO))
        .head
        .expect("the head is claimed");

    assert_eq!(queue.close(), std::slice::from_ref(&claimed));

    assert!(!queue.take_head(&claimed));
    queue.release(target());
    assert_eq!(queue.push(waiting("late", at(Duration::ZERO))), None);
    assert!(!queue.claim_empty(target(), at(Duration::ZERO)));
    assert!(
        queue
            .claim_head(target(), at(Duration::ZERO))
            .head
            .is_none()
    );
}

#[test]
fn a_second_report_of_a_sender_replaces_its_waiting_one_in_place_with_its_own_time() {
    let queue = DirectiveQueue::default();
    queue.push(waiting("directive", at(Duration::ZERO)));
    queue.push_report(report_from(2, "first", at(Duration::ZERO)));
    queue.push(waiting("after", at(Duration::ZERO)));

    let pushed = queue.push_report(report_from(2, "second", at(Duration::from_secs(5))));

    assert_eq!(
        pushed,
        ReportPush::Replaced {
            position: 2,
            replaced: report_from(2, "first", at(Duration::ZERO)),
        }
    );
    assert_eq!(queue.waiting_count(target()), 3);
    let claim = queue.claim_head(target(), at(WAIT_LIMIT));
    let expired: Vec<&str> = claim
        .expired
        .iter()
        .map(|gone| gone.text.as_str())
        .collect();
    assert_eq!(expired, ["directive"]);
    assert_eq!(claim.head.map(|head| head.text), Some("second".to_owned()));
}

#[test]
fn a_report_never_replaces_the_claimed_head_and_joins_the_end_instead() {
    let queue = DirectiveQueue::default();
    queue.push_report(report_from(2, "first", at(Duration::ZERO)));
    let claim = queue.claim_head(target(), at(Duration::ZERO));
    let claimed = claim.head.expect("claimed head");
    assert_eq!(claimed.text, "first");

    let pushed = queue.push_report(report_from(2, "second", at(Duration::ZERO)));

    assert_eq!(pushed, ReportPush::Added { position: 2 });
    assert!(queue.take_head(&claimed));
}

#[test]
fn reports_take_at_most_their_share_and_directives_keep_the_rest() {
    let queue = DirectiveQueue::default();
    for sender in 0..u128::try_from(REPORT_SHARE).expect("small share") {
        let pushed = queue.push_report(report_from(100 + sender, "done", at(Duration::ZERO)));
        assert!(matches!(pushed, ReportPush::Added { .. }));
    }

    let refused = queue.push_report(report_from(999, "late", at(Duration::ZERO)));
    let directive_place = queue.push(waiting("from above", at(Duration::ZERO)));

    assert_eq!(refused, ReportPush::Refused(ReportRoom::NoRoomForReports));
    assert_eq!(directive_place, Some(REPORT_SHARE + 1));
}

#[test]
fn a_full_queue_refuses_a_new_report_but_a_replacement_still_fits() {
    let queue = DirectiveQueue::default();
    queue.push_report(report_from(2, "first", at(Duration::ZERO)));
    for _ in 1..QUEUE_LIMIT {
        queue.push(waiting("directive", at(Duration::ZERO)));
    }

    assert_eq!(
        queue.report_room(target(), AgentId::from_u128(3)),
        ReportRoom::Full
    );
    assert_eq!(
        queue.report_room(target(), AgentId::from_u128(2)),
        ReportRoom::Replace
    );
}

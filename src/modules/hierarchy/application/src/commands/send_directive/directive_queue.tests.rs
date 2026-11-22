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
        sender: AgentId::from_u128(1),
        target: target(),
        text: text.to_owned(),
        accepted_at,
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

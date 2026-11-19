// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

const AGENT: u128 = 1;
const OTHER_AGENT: u128 = 2;

fn id(value: u128) -> AgentId {
    AgentId::from_u128(value)
}

fn at_seconds(seconds: i64) -> Timestamp {
    Timestamp::from_unix_millis(seconds * 1000)
}

#[test]
fn the_third_refusal_within_a_minute_returns_true_once() {
    let mut tracker = AutoHoldTracker::default();
    assert!(!tracker.record_limit_refusal(id(AGENT), at_seconds(0)));
    assert!(!tracker.record_limit_refusal(id(AGENT), at_seconds(10)));
    assert!(tracker.record_limit_refusal(id(AGENT), at_seconds(20)));
    // The history is cleared: the 4th and 5th do not fire again.
    assert!(!tracker.record_limit_refusal(id(AGENT), at_seconds(21)));
    assert!(!tracker.record_limit_refusal(id(AGENT), at_seconds(22)));
    assert!(tracker.record_limit_refusal(id(AGENT), at_seconds(23)));
}

#[test]
fn refusals_spread_over_more_than_a_minute_do_not_fire() {
    let mut tracker = AutoHoldTracker::default();
    for seconds in [0, 30, 61, 91] {
        assert!(!tracker.record_limit_refusal(id(AGENT), at_seconds(seconds)));
    }
}

#[test]
fn each_agent_has_its_own_history() {
    let mut tracker = AutoHoldTracker::default();
    assert!(!tracker.record_limit_refusal(id(AGENT), at_seconds(0)));
    assert!(!tracker.record_limit_refusal(id(OTHER_AGENT), at_seconds(1)));
    assert!(!tracker.record_limit_refusal(id(AGENT), at_seconds(2)));
    assert!(!tracker.record_limit_refusal(id(OTHER_AGENT), at_seconds(3)));
    assert!(tracker.record_limit_refusal(id(AGENT), at_seconds(4)));
}

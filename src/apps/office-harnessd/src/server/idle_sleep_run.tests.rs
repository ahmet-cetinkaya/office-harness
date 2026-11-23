// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::{Duration, Instant};

use kernel::AgentId;

use super::RoundState;

const MINUTE: Duration = Duration::from_millis(100);

#[test]
fn the_setting_off_gives_no_rule_and_the_first_value_counts_from_the_start() {
    let started_at = Instant::now();
    let mut state = RoundState::new(started_at);
    assert_eq!(state.observe(0, MINUTE), None);

    let mut state = RoundState::new(started_at);
    let rule = state.observe(5, MINUTE).expect("idle sleep is on");
    assert_eq!(rule.idle_time, MINUTE * 5);
    assert_eq!(rule.counted_from, started_at);
}

#[test]
fn turning_the_setting_on_or_changing_it_restarts_the_count() {
    let started_at = Instant::now();
    let mut state = RoundState::new(started_at);
    assert_eq!(state.observe(0, MINUTE), None);
    let turned_on = state.observe(30, MINUTE).expect("idle sleep is on");
    assert!(turned_on.counted_from > started_at);

    let unchanged = state.observe(30, MINUTE).expect("idle sleep is on");
    assert_eq!(unchanged.counted_from, turned_on.counted_from);

    let changed = state.observe(45, MINUTE).expect("idle sleep is on");
    assert!(changed.counted_from > turned_on.counted_from);
}

#[test]
fn a_failed_sleep_bars_the_agent_until_a_prompted_wait_after_the_failure() {
    let agent = AgentId::from_u128(4);
    let mut state = RoundState::new(Instant::now());
    assert!(!state.bars(agent, Some(Instant::now())));

    let failed_at = Instant::now();
    state.failed.insert(agent, failed_at);
    std::thread::sleep(Duration::from_millis(20));
    assert!(state.bars(agent, None));
    assert!(state.bars(agent, Some(failed_at)));
    assert!(!state.bars(agent, Some(Instant::now())));
    assert!(!state.failed.contains_key(&agent));
}

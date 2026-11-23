// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

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
    assert_eq!(gate.quiet_for(agent()), None);
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

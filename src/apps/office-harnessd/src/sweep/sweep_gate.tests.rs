// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

use super::*;

const SHORT_WAIT: Duration = Duration::from_millis(100);

#[tokio::test]
async fn a_gate_without_a_sweep_lets_every_request_pass() {
    let gate = SweepGate::default();

    assert_eq!(gate.state(), SweepState::Done);
    assert_eq!(gate.wait_until_done().await, Ok(()));
}

#[tokio::test]
async fn a_request_waits_while_the_sweep_runs_and_passes_when_it_ends() {
    let gate = SweepGate::new(true);
    let waiting = tokio::spawn({
        let gate = gate.clone();
        async move { gate.wait_until_done().await }
    });

    assert_eq!(gate.state(), SweepState::Running);
    tokio::time::sleep(SHORT_WAIT).await;
    assert!(!waiting.is_finished());
    gate.finish();

    assert_eq!(waiting.await.expect("task"), Ok(()));
    assert_eq!(gate.state(), SweepState::Done);
}

#[tokio::test]
async fn a_stop_during_the_sweep_refuses_the_waiting_request() {
    let gate = SweepGate::new(true);
    let waiting = tokio::spawn({
        let gate = gate.clone();
        async move { gate.wait_until_done().await }
    });

    gate.stop();

    assert_eq!(waiting.await.expect("task"), Err(ServerStopping));
}

#[tokio::test]
async fn a_finished_gate_stays_finished_after_a_stop() {
    let gate = SweepGate::new(true);
    gate.finish();

    gate.stop();

    assert_eq!(gate.wait_until_done().await, Ok(()));
}

#[test]
fn the_event_names_the_state() {
    let frame = sweep_event(SweepState::Running).encode();

    assert!(frame.contains("office.sweep"), "{frame}");
    assert!(frame.contains("\"state\":\"running\""), "{frame}");
}

// SPDX-License-Identifier: AGPL-3.0-or-later

use std::panic::{AssertUnwindSafe, catch_unwind};

use super::*;

#[test]
fn a_panic_in_the_sweep_task_still_opens_the_gate_and_publishes_done() {
    let gate = SweepGate::new(true);
    let events = EventHub::new();
    let mut subscriber = events.subscribe();

    let panicked = catch_unwind(AssertUnwindSafe(|| {
        let _finish = FinishOnDrop {
            gate: gate.clone(),
            events: events.clone(),
        };
        panic!("the notifier failed");
    }));

    assert!(panicked.is_err());
    assert_eq!(gate.state(), SweepState::Done);
    let published = subscriber.try_recv().expect("an event was published");
    assert!(published.contains("done"), "{published}");
}

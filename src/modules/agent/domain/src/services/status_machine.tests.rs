// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

use AgentStatus::{
    Asleep, Exited, Failed, Interrupted, Starting, Stopped, WaitingForInput, Working,
};
use StatusEvent::{
    ExitedWithCodeZero, ExitedWithFailure, FirstOutput, Output, PutToSleep, QuietPeriodElapsed,
    ServerStopped, Start, StoppedByUser,
};

/// Every transition of the data model's state machine, written out once.
fn allowed_transitions() -> Vec<(AgentStatus, StatusEvent, AgentStatus)> {
    let mut transitions = vec![
        (Starting, FirstOutput, Working),
        (Working, QuietPeriodElapsed, WaitingForInput),
        (WaitingForInput, Output, Working),
        (WaitingForInput, PutToSleep, Asleep),
        (Asleep, StoppedByUser, Stopped),
    ];
    for running in AgentStatus::RUNNING {
        transitions.push((running, ExitedWithCodeZero, Exited));
        transitions.push((running, ExitedWithFailure, Failed));
        transitions.push((running, StoppedByUser, Stopped));
        transitions.push((running, ServerStopped, Interrupted));
    }
    for finished in [Exited, Failed, Stopped, Interrupted, Asleep] {
        transitions.push((finished, Start, Starting));
    }
    transitions
}

#[test]
fn allows_exactly_the_transitions_of_the_data_model() {
    let allowed = allowed_transitions();
    assert_eq!(allowed.len(), 22);

    for status in AgentStatus::ALL {
        for event in StatusEvent::ALL {
            let expected = allowed
                .iter()
                .find(|(from, trigger, _)| *from == status && *trigger == event)
                .map(|(_, _, next)| *next)
                .ok_or(AgentError::InvalidTransition {
                    from: status,
                    event,
                });
            assert_eq!(next_status(status, event), expected, "{status} + {event}");
        }
    }
}

#[test]
fn only_an_explicit_start_moves_an_interrupted_agent() {
    for event in StatusEvent::ALL {
        let moved = next_status(Interrupted, event).is_ok();
        assert_eq!(moved, event == Start, "{event}");
    }
}

#[test]
fn error_message_names_the_status_and_the_event() {
    let error = next_status(Stopped, Output).expect_err("not allowed");
    assert_eq!(
        error.to_string(),
        "an agent that is stopped cannot take the event output"
    );
}

#[test]
fn only_a_waiting_agent_goes_to_sleep() {
    for status in AgentStatus::ALL {
        let moved = next_status(status, PutToSleep).is_ok();
        assert_eq!(moved, status == WaitingForInput, "{status}");
    }
}

#[test]
fn an_asleep_agent_is_not_running_and_takes_only_a_start_or_a_stop() {
    assert!(!Asleep.is_running());
    for event in StatusEvent::ALL {
        let moved = next_status(Asleep, event).is_ok();
        assert_eq!(moved, matches!(event, Start | StoppedByUser), "{event}");
    }
}

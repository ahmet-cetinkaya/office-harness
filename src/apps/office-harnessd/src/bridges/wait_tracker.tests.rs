// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

const RUN: RunId = RunId(4);
const QUIET: Duration = Duration::from_secs(5);
const BOUND: Duration = Duration::from_secs(15);
const REDRAW: Duration = Duration::from_secs(5);

fn agent() -> AgentId {
    AgentId::from_u128(9)
}

fn at(start: Instant, seconds: u64) -> Instant {
    start + Duration::from_secs(seconds)
}

/// A tracker whose agent finished its first wait, ready for a redraw.
fn waiting_tracker(start: Instant) -> WaitTracker {
    let tracker = WaitTracker::new(QUIET);
    tracker.session_started(agent(), Some(RUN));
    tracker.classify_at(agent(), StatusChange::ToWorking, start);
    tracker.classify_at(agent(), StatusChange::ToWaitingForInput, at(start, 1));
    tracker
}

#[test]
fn the_first_wait_of_a_run_is_prompted() {
    let start = Instant::now();
    let tracker = WaitTracker::new(QUIET);
    tracker.session_started(agent(), Some(RUN));
    assert_eq!(
        tracker.classify_at(agent(), StatusChange::ToWorking, start),
        Change::Prompted
    );
    assert_eq!(
        tracker.classify_at(agent(), StatusChange::ToWaitingForInput, at(start, 1)),
        Change::Prompted
    );
}

#[test]
fn a_redraw_is_unprompted_both_ways() {
    let start = Instant::now();
    let tracker = waiting_tracker(start);
    assert_eq!(
        tracker.classify_at(agent(), StatusChange::ToWorking, at(start, 20)),
        Change::Unprompted
    );
    assert_eq!(
        tracker.classify_at(
            agent(),
            StatusChange::ToWaitingForInput,
            at(start, 20) + REDRAW
        ),
        Change::Unprompted
    );
}

#[test]
fn a_person_input_makes_the_next_change_prompted() {
    let start = Instant::now();
    let tracker = waiting_tracker(start);
    tracker.wait_end(agent());
    assert_eq!(
        tracker.classify_at(agent(), StatusChange::ToWorking, at(start, 20)),
        Change::Prompted
    );
}

#[test]
fn office_text_makes_the_next_change_prompted() {
    let start = Instant::now();
    let tracker = waiting_tracker(start);
    tracker.wait_end(agent());
    assert_eq!(
        tracker.classify_at(agent(), StatusChange::ToWaitingForInput, at(start, 20)),
        Change::Prompted
    );
}

#[test]
fn a_session_start_resets_the_state_and_counts_as_a_wait_end() {
    let start = Instant::now();
    let tracker = waiting_tracker(start);
    tracker.session_started(agent(), Some(RunId(5)));
    assert_eq!(tracker.run(agent()), Some(RunId(5)));
    assert_eq!(tracker.working_since(agent()), None);
    assert_eq!(tracker.last_prompted_wait(agent()), None);
    assert_eq!(
        tracker.classify_at(agent(), StatusChange::ToWaitingForInput, at(start, 20)),
        Change::Prompted
    );
}

#[test]
fn a_session_end_resets_the_state_and_counts_as_a_wait_end() {
    let start = Instant::now();
    let tracker = waiting_tracker(start);
    tracker.session_ended(agent());
    assert_eq!(tracker.run(agent()), None);
    assert_eq!(tracker.working_since(agent()), None);
    assert_eq!(tracker.last_prompted_wait(agent()), None);
    assert_eq!(
        tracker.classify_at(agent(), StatusChange::ToWorking, at(start, 20)),
        Change::Prompted
    );
}

#[test]
fn a_stretch_of_working_longer_than_the_bound_makes_the_return_prompted() {
    let start = Instant::now();
    let tracker = waiting_tracker(start);
    let working = at(start, 20);
    tracker.classify_at(agent(), StatusChange::ToWorking, working);
    assert_eq!(
        tracker.classify_at(
            agent(),
            StatusChange::ToWaitingForInput,
            working + BOUND + Duration::from_millis(1)
        ),
        Change::Prompted
    );
}

#[test]
fn a_stretch_of_working_within_the_bound_leaves_the_return_unprompted() {
    let start = Instant::now();
    let tracker = waiting_tracker(start);
    let working = at(start, 20);
    tracker.classify_at(agent(), StatusChange::ToWorking, working);
    assert_eq!(
        tracker.classify_at(agent(), StatusChange::ToWaitingForInput, working + BOUND),
        Change::Unprompted
    );
}

#[test]
fn a_longer_quiet_period_and_a_shorter_extra_move_the_bound() {
    let start = Instant::now();
    let tracker = WaitTracker::new(Duration::from_secs(20));
    tracker.set_work_bound_extra(Duration::from_secs(1));
    tracker.session_started(agent(), Some(RUN));
    tracker.classify_at(agent(), StatusChange::ToWorking, start);
    tracker.classify_at(agent(), StatusChange::ToWaitingForInput, at(start, 1));
    tracker.classify_at(agent(), StatusChange::ToWorking, at(start, 100));
    assert_eq!(
        tracker.classify_at(agent(), StatusChange::ToWaitingForInput, at(start, 122)),
        Change::Prompted
    );
}

#[test]
fn the_last_prompted_wait_moves_only_on_prompted_waits() {
    let start = Instant::now();
    let tracker = waiting_tracker(start);
    assert_eq!(tracker.last_prompted_wait(agent()), Some(at(start, 1)));

    tracker.classify_at(agent(), StatusChange::ToWorking, at(start, 20));
    tracker.classify_at(agent(), StatusChange::ToWaitingForInput, at(start, 22));
    assert_eq!(tracker.last_prompted_wait(agent()), Some(at(start, 1)));

    tracker.wait_end(agent());
    tracker.classify_at(agent(), StatusChange::ToWorking, at(start, 30));
    tracker.classify_at(agent(), StatusChange::ToWaitingForInput, at(start, 32));
    assert_eq!(tracker.last_prompted_wait(agent()), Some(at(start, 32)));
}

#[test]
fn a_change_to_working_stores_the_start_of_the_stretch() {
    let start = Instant::now();
    let tracker = waiting_tracker(start);
    tracker.classify_at(agent(), StatusChange::ToWorking, at(start, 20));
    assert_eq!(tracker.working_since(agent()), Some(at(start, 20)));
}

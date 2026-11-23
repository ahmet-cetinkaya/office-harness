// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::value_objects::{AgentCommand, DisplayName, MachineId, Objective, TempMark};
use kernel::Timestamp;

use super::*;

const LIMIT: Duration = Duration::from_secs(300);

fn ago(now: Instant, seconds: u64) -> Instant {
    now.checked_sub(Duration::from_secs(seconds))
        .expect("the process has run long enough")
}

#[test]
fn the_quiet_time_of_a_running_temp_is_the_time_since_its_last_output_or_input() {
    let now = Instant::now() + Duration::from_secs(1000);

    let since = quiet_since(now, Some(Duration::from_secs(40)), None, None);

    assert_eq!(since, ago(now, 40));
}

#[test]
fn a_hold_a_running_child_or_an_item_in_needs_you_restarts_the_quiet_time() {
    let now = Instant::now() + Duration::from_secs(1000);
    let blocked = ago(now, 10);

    let since = quiet_since(now, Some(Duration::from_secs(40)), None, Some(blocked));

    assert_eq!(since, blocked);
}

#[test]
fn an_old_block_does_not_shorten_a_longer_quiet_time_it_does_not_cover() {
    let now = Instant::now() + Duration::from_secs(1000);

    let since = quiet_since(now, Some(Duration::from_secs(5)), None, Some(ago(now, 60)));

    assert_eq!(since, ago(now, 5));
}

#[test]
fn an_asleep_temp_keeps_the_moment_of_the_last_round() {
    let now = Instant::now() + Duration::from_secs(1000);
    let known = ago(now, 90);

    assert_eq!(quiet_since(now, None, Some(known), None), known);
}

#[test]
fn a_temp_never_seen_before_has_been_quiet_since_now() {
    let now = Instant::now() + Duration::from_secs(1000);

    assert_eq!(quiet_since(now, None, None, None), now);
}

#[test]
fn the_idle_limit_is_reached_at_exactly_its_length_and_not_before() {
    let now = Instant::now() + Duration::from_secs(1000);

    let just_short = ago(now, LIMIT.as_secs()) + Duration::from_millis(1);
    assert!(!idle_limit_reached(now, just_short, LIMIT));
    assert!(idle_limit_reached(now, ago(now, LIMIT.as_secs()), LIMIT));
}

#[test]
fn an_exit_code_is_an_exit_and_anything_else_a_failure_with_its_reason() {
    assert_eq!(
        exit_reason_of(Some(&ExitResult::Code(1))),
        EndReason::Exited { code: 1 }
    );
    assert_eq!(
        exit_reason_of(Some(&ExitResult::SpawnError("no such command".to_owned()))),
        EndReason::Failed {
            reason: "no such command".to_owned()
        }
    );
    assert_eq!(
        exit_reason_of(Some(&ExitResult::NotSignedIn("sign in first".to_owned()))),
        EndReason::Failed {
            reason: "sign in first".to_owned()
        }
    );
    assert_eq!(
        exit_reason_of(None),
        EndReason::Failed {
            reason: NO_EXIT_RESULT.to_owned()
        }
    );
}

#[test]
fn the_board_reports_the_quiet_time_of_the_last_round() {
    let board = QuietBoard::default();
    let agent = AgentId::from_u128(1);
    assert_eq!(board.quiet_for(agent), None);

    board.replace(HashMap::from([(agent, Instant::now())]));

    assert!(board.quiet_for(agent).is_some());
    board.replace(HashMap::new());
    assert_eq!(board.quiet_for(agent), None);
}

#[test]
fn the_idle_end_names_the_status_the_round_read_so_a_new_task_that_took_the_order_wins() {
    let mut temp = Agent::new(
        AgentId::from_u128(1),
        DisplayName::parse("name", "quiet").expect("name"),
        AgentCommand::parse("claude").expect("command"),
        Some(std::env::temp_dir()),
        None,
        MachineId::generate(),
        Timestamp::from_unix_millis(0),
    )
    .expect("agent");
    temp.status = AgentStatus::WaitingForInput;
    temp.temp = Some(TempMark {
        objective: Objective::new("job").expect("objective"),
        creator: Actor::User,
    });

    let order = idle_end_of(&temp, 5);

    assert_eq!(order.seen, Some(AgentStatus::WaitingForInput));
    assert_eq!(order.reason, EndReason::NoOutput { minutes: 5 });
}

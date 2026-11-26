// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::mpsc;

use agent_domain::value_objects::{AgentCommand, DisplayName, MachineId, Objective, TempMark};
use audit_application::common::errors::AuditError;
use audit_application::common::interfaces::ActionLog;
use audit_domain::{ActionEntry, ActionRecord};
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

/// An action record whose reads are reported on a channel, so a test sees
/// when the record is read.
struct WatchedLog {
    entries: Mutex<Vec<ActionEntry>>,
    reads: Mutex<mpsc::Sender<()>>,
}

impl ActionLog for WatchedLog {
    fn append(&self, record: &ActionRecord) -> Result<ActionEntry, AuditError> {
        let mut entries = self.entries.lock().expect("entries");
        let entry = ActionEntry {
            id: i64::try_from(entries.len()).expect("id"),
            record: record.clone(),
        };
        entries.push(entry.clone());
        Ok(entry)
    }

    fn list_for_agent(&self, _agent_id: AgentId) -> Result<Vec<ActionEntry>, AuditError> {
        let _ = self.reads.lock().expect("reads").send(());
        Ok(self.entries.lock().expect("entries").clone())
    }

    fn list_office_level(&self) -> Result<Vec<ActionEntry>, AuditError> {
        Ok(Vec::new())
    }
}

#[test]
fn a_stop_that_saved_the_status_but_not_yet_its_entry_still_names_its_actor() {
    let temp = AgentId::from_u128(1);
    let boss = AgentId::from_u128(2);
    let (reads, was_read) = mpsc::channel();
    let log = Arc::new(WatchedLog {
        entries: Mutex::default(),
        reads: Mutex::new(reads),
    });
    let actions = ListActionsHandler::new(Arc::clone(&log) as Arc<dyn ActionLog>);
    let status_order = StatusOrder::default();
    // The stop request: it holds the order from the saved status to the entry.
    let stop_in_flight = status_order.lock().expect("order");
    let watcher = {
        let (actions, status_order) = (actions.clone(), Arc::clone(&status_order));
        std::thread::spawn(move || stop_actor(&actions, &status_order, temp))
    };

    let read_before_the_entry = was_read.recv_timeout(Duration::from_millis(200)).is_ok();
    log.append(&ActionRecord {
        actor: Actor::Agent(boss),
        target_agent_id: Some(temp),
        action: ActionKind::Stop,
        result: ActionResult::Ok,
        detail: None,
        actor_label: None,
        at: Timestamp::from_unix_millis(0),
    })
    .expect("append");
    drop(stop_in_flight);

    assert!(!read_before_the_entry);
    assert_eq!(watcher.join().expect("watcher"), Actor::Agent(boss));
}

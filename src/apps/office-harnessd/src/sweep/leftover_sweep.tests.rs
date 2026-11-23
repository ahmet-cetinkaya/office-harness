// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use agent_application::common::models::SessionProcess;
use agent_domain::value_objects::SessionId;
use audit_domain::ActionResult;
use terminal_application::common::interfaces::{EndOutcome, ProcessHandle, ReadError};
use terminal_application::common::models::ProcessIdentity;

use super::*;
use crate::bridges::SystemClock;

const GRACE: Duration = Duration::from_millis(10);
const SLOW_EXIT: Duration = Duration::from_millis(200);
const BOOT: &str = "boot";
const HASH: [u8; 32] = [3; 32];
const STORED_START: u64 = 500;

fn locked<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// What a fake process does when the sweep asks it to end.
#[derive(Clone)]
enum Behavior {
    /// Ends on the request after `exit_after`.
    Ends { exit_after: Duration },
    /// Matches, and the OS refuses to end it.
    Refuses(String),
    /// Matches no stored identity: another start time.
    Unrelated,
}

#[derive(Clone)]
struct FakeInspector {
    behaviors: Arc<Mutex<HashMap<u32, Behavior>>>,
}

struct FakeHandle {
    pid: u32,
    started_at: u64,
    behavior: Behavior,
}

impl ProcessHandle for FakeHandle {
    fn identity(&self) -> Result<ProcessIdentity, ReadError> {
        Ok(ProcessIdentity {
            pid: self.pid,
            started_at: self.started_at,
            command_hash: HASH,
            command_display: "fake".to_owned(),
            boot: BOOT.to_owned(),
        })
    }

    fn request_end(&self) -> Result<EndOutcome, String> {
        match &self.behavior {
            Behavior::Refuses(error) => Err(error.clone()),
            Behavior::Ends { .. } | Behavior::Unrelated => Ok(EndOutcome::Delivered),
        }
    }

    fn force_end(&self) -> Result<EndOutcome, String> {
        Ok(EndOutcome::Delivered)
    }

    fn wait_for_exit(&self, _timeout: Duration) -> bool {
        if let Behavior::Ends { exit_after } = &self.behavior {
            std::thread::sleep(*exit_after);
            return true;
        }
        false
    }
}

impl ProcessInspector for FakeInspector {
    fn open(&self, pid: u32) -> Option<Box<dyn ProcessHandle>> {
        let behavior = locked(&self.behaviors).get(&pid)?.clone();
        let started_at = match behavior {
            Behavior::Unrelated => STORED_START + 1,
            Behavior::Ends { .. } | Behavior::Refuses(_) => STORED_START,
        };
        Some(Box::new(FakeHandle {
            pid,
            started_at,
            behavior,
        }))
    }

    fn boot(&self) -> Option<String> {
        Some(BOOT.to_owned())
    }

    fn descendants(&self, _root: u32) -> Vec<u32> {
        Vec::new()
    }
}

#[derive(Default)]
struct RecordingLedger {
    entries: Mutex<Vec<(i64, ActionRecord)>>,
}

impl SweepLedger for RecordingLedger {
    fn record(&self, identity_id: i64, record: &ActionRecord) -> Result<ActionEntry, String> {
        locked(&self.entries).push((identity_id, record.clone()));
        Ok(ActionEntry {
            id: identity_id,
            record: record.clone(),
        })
    }
}

#[derive(Default)]
struct RecordingNotifier {
    kept: Mutex<Vec<(AgentId, String)>>,
}

impl LeftoverNotifier for RecordingNotifier {
    fn leftover_kept(&self, agent_id: AgentId, reason: &str) {
        locked(&self.kept).push((agent_id, reason.to_owned()));
    }
}

struct Fixture {
    sweep: LeftoverSweep,
    ledger: Arc<RecordingLedger>,
    notifier: Arc<RecordingNotifier>,
    _folder: tempfile::TempDir,
}

fn fixture(behaviors: Vec<(u32, Behavior)>) -> Fixture {
    let folder = tempfile::tempdir().expect("folder");
    let config = crate::config::Config {
        data_folder: folder.path().join("office"),
    };
    let log = crate::bootstrap::open_log(&config).expect("log");
    let ledger = Arc::new(RecordingLedger::default());
    let notifier = Arc::new(RecordingNotifier::default());
    let sweep = LeftoverSweep {
        inspector: Arc::new(FakeInspector {
            behaviors: Arc::new(Mutex::new(behaviors.into_iter().collect())),
        }),
        ledger: Arc::clone(&ledger) as Arc<dyn SweepLedger>,
        notifier: Arc::clone(&notifier) as Arc<dyn LeftoverNotifier>,
        clock: Arc::new(SystemClock),
        events: EventHub::new(),
        alerts: StorageAlerts::new(log.clone()),
        log,
    };
    Fixture {
        sweep,
        ledger,
        notifier,
        _folder: folder,
    }
}

fn leftover(id: i64, pid: u32, agent_id: AgentId, command: &str) -> Leftover {
    Leftover {
        stored: StoredProcess {
            id,
            session_id: SessionId::generate(),
            kind: agent_application::common::models::ProcessKind::Cli,
            process: SessionProcess {
                pid,
                started_at: STORED_START,
                command_hash: HASH,
                command_display: command.to_owned(),
                boot: BOOT.to_owned(),
            },
        },
        agent_id,
    }
}

fn details(ledger: &RecordingLedger) -> Vec<(i64, ActionRecord)> {
    let mut entries = locked(&ledger.entries).clone();
    entries.sort_by_key(|(id, _)| *id);
    entries
}

#[tokio::test]
async fn each_identity_gets_one_entry_that_starts_with_its_row_id() {
    let agent = AgentId::generate();
    let fixture = fixture(vec![
        (
            11,
            Behavior::Ends {
                exit_after: Duration::ZERO,
            },
        ),
        (12, Behavior::Unrelated),
    ]);

    fixture
        .sweep
        .run(
            vec![
                leftover(1, 11, agent, "sleep 600"),
                leftover(2, 12, agent, "sleep 600"),
                leftover(3, 13, agent, "gone"),
            ],
            GRACE,
        )
        .await;

    let entries = details(&fixture.ledger);
    let texts: Vec<_> = entries
        .iter()
        .map(|(_, record)| record.detail.clone().expect("detail"))
        .collect();
    assert_eq!(entries.len(), 3);
    assert!(
        texts[0].starts_with("identity 1: ended: pid 11, session "),
        "{texts:?}"
    );
    assert!(texts[0].ends_with(", sleep 600"), "{texts:?}");
    assert!(
        texts[1].starts_with("identity 2: left: other start time: pid 12"),
        "{texts:?}"
    );
    assert!(
        texts[2].starts_with("identity 3: left: not running: pid 13"),
        "{texts:?}"
    );
    for (_, record) in &entries {
        assert_eq!(record.actor, Actor::Office);
        assert_eq!(record.action, ActionKind::EndLeftover);
        assert_eq!(record.target_agent_id, Some(agent));
        assert_eq!(record.result, ActionResult::Ok);
    }
    assert!(locked(&fixture.notifier.kept).is_empty());
}

#[tokio::test]
async fn the_command_line_in_an_entry_is_in_visible_form() {
    let fixture = fixture(vec![(
        11,
        Behavior::Ends {
            exit_after: Duration::ZERO,
        },
    )]);

    fixture
        .sweep
        .run(
            vec![leftover(
                1,
                11,
                AgentId::generate(),
                "echo \u{1b}[31m\u{202e}x",
            )],
            GRACE,
        )
        .await;

    let entries = details(&fixture.ledger);
    let text = entries[0].1.detail.clone().expect("detail");
    assert!(text.ends_with("echo \\u{1b}[31m\\u{202e}x"), "{text}");
    assert!(
        !text.contains('\u{1b}') && !text.contains('\u{202e}'),
        "{text}"
    );
}

#[tokio::test]
async fn a_process_the_os_will_not_end_fails_its_entry_and_notifies_its_agent_once() {
    let stuck = AgentId::generate();
    let fine = AgentId::generate();
    let fixture = fixture(vec![
        (
            11,
            Behavior::Refuses("Operation not permitted (os error 1)".to_owned()),
        ),
        (
            12,
            Behavior::Refuses("Operation not permitted (os error 1)".to_owned()),
        ),
        (
            13,
            Behavior::Ends {
                exit_after: Duration::ZERO,
            },
        ),
    ]);

    fixture
        .sweep
        .run(
            vec![
                leftover(1, 11, stuck, "cli"),
                leftover(2, 12, stuck, "helper"),
                leftover(3, 13, fine, "cli"),
            ],
            GRACE,
        )
        .await;

    let entries = details(&fixture.ledger);
    assert_eq!(
        entries[0].1.result,
        ActionResult::Failed("Operation not permitted (os error 1)".to_owned())
    );
    assert!(
        entries[0]
            .1
            .detail
            .clone()
            .expect("detail")
            .starts_with("identity 1: could not end: Operation not permitted (os error 1): pid 11")
    );
    let kept = locked(&fixture.notifier.kept).clone();
    assert_eq!(
        kept,
        [(stuck, "Operation not permitted (os error 1)".to_owned())]
    );
}

#[tokio::test]
async fn all_identities_are_swept_at_the_same_time() {
    const COUNT: u32 = 10;
    let behaviors = (0..COUNT)
        .map(|pid| {
            (
                pid,
                Behavior::Ends {
                    exit_after: SLOW_EXIT,
                },
            )
        })
        .collect();
    let fixture = fixture(behaviors);
    let agent = AgentId::generate();
    let leftovers = (0..COUNT)
        .map(|pid| leftover(i64::from(pid), pid, agent, "cli"))
        .collect();
    let started = Instant::now();

    fixture.sweep.run(leftovers, GRACE).await;

    assert_eq!(locked(&fixture.ledger.entries).len(), COUNT as usize);
    assert!(
        started.elapsed() < SLOW_EXIT * (COUNT / 2),
        "one after the other would take {:?}",
        SLOW_EXIT * COUNT
    );
}

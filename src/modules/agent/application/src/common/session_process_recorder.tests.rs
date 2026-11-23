// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::entities::Session;
use agent_domain::value_objects::OfficeAdditions;
use kernel::{Actor, AgentId, Timestamp};

use super::*;
use crate::test_support::{Office, process_running};

const CLI_PID: u32 = 100;
const HELPER_PID: u32 = 200;
const STARTED_AT: u64 = 5_000;

struct Fixture {
    office: Office,
    recorder: SessionProcessRecorder,
    session: Session,
}

fn fixture() -> Fixture {
    let office = Office::new();
    let session = Session::begin(
        SessionId::generate(),
        AgentId::generate(),
        None,
        None,
        OfficeAdditions::default(),
        Timestamp::from_unix_millis(1),
    );
    office.sessions.insert(&session).expect("session");
    office.probe.set_cli(session.agent_id, CLI_PID);
    office
        .probe
        .set_read(CLI_PID, Ok(process_running(CLI_PID, STARTED_AT, "claude")));
    let recorder = office.process_recorder();
    Fixture {
        office,
        recorder,
        session,
    }
}

fn start_helper(fixture: &Fixture, pid: u32, command: &str) {
    fixture.office.probe.set_read(
        pid,
        Ok(process_running(pid, STARTED_AT + u64::from(pid), command)),
    );
}

#[test]
fn the_cli_identity_is_stored_as_open_cli_row() {
    let fixture = fixture();

    fixture.recorder.record_cli(&fixture.session);

    let rows = fixture.office.processes.rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, ProcessKind::Cli);
    assert_eq!(rows[0].process.pid, CLI_PID);
}

#[test]
fn a_command_line_that_is_empty_at_first_is_read_again() {
    let fixture = fixture();
    fixture.office.probe.make_next_reads_unreadable(2);

    fixture.recorder.record_cli(&fixture.session);

    assert_eq!(fixture.office.processes.rows().len(), 1);
}

#[test]
fn a_cli_that_stays_unreadable_stores_nothing() {
    let fixture = fixture();
    fixture
        .office
        .probe
        .set_read(CLI_PID, Err(ProbeError::NotReadable));

    fixture.recorder.record_cli(&fixture.session);

    assert_eq!(fixture.office.processes.rows(), Vec::new());
}

#[test]
fn a_new_helper_is_stored_once_however_often_it_is_scanned() {
    let fixture = fixture();
    start_helper(&fixture, HELPER_PID, "node server.js");
    fixture.office.probe.set_descendants(&[HELPER_PID]);

    fixture.recorder.scan();
    fixture.recorder.scan();

    let rows = fixture.office.processes.rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, ProcessKind::Helper);
}

#[test]
fn a_helper_that_left_the_tree_stays_stored_while_it_matches() {
    let fixture = fixture();
    start_helper(&fixture, HELPER_PID, "node server.js");
    fixture.office.probe.set_descendants(&[HELPER_PID]);
    fixture.recorder.scan();

    fixture.office.probe.set_descendants(&[]);
    fixture.recorder.scan();

    assert_eq!(fixture.office.processes.rows().len(), 1);
}

#[test]
fn a_helper_that_ended_is_dropped() {
    let fixture = fixture();
    start_helper(&fixture, HELPER_PID, "node server.js");
    fixture.office.probe.set_descendants(&[HELPER_PID]);
    fixture.recorder.scan();

    fixture
        .office
        .probe
        .set_read(HELPER_PID, Err(ProbeError::NotRunning));
    fixture.recorder.scan();

    assert_eq!(fixture.office.processes.rows(), Vec::new());
}

#[test]
fn a_helper_that_cannot_be_read_stays_stored() {
    let fixture = fixture();
    start_helper(&fixture, HELPER_PID, "node server.js");
    fixture.office.probe.set_descendants(&[HELPER_PID]);
    fixture.recorder.scan();

    fixture
        .office
        .probe
        .set_read(HELPER_PID, Err(ProbeError::NotReadable));
    fixture.recorder.scan();

    assert_eq!(fixture.office.processes.rows().len(), 1);
}

#[test]
fn a_helper_whose_id_went_to_another_process_is_replaced() {
    let fixture = fixture();
    start_helper(&fixture, HELPER_PID, "node server.js");
    fixture.office.probe.set_descendants(&[HELPER_PID]);
    fixture.recorder.scan();

    fixture.office.probe.set_read(
        HELPER_PID,
        Ok(process_running(HELPER_PID, STARTED_AT + 9_999, "python")),
    );
    fixture.recorder.scan();

    let rows = fixture.office.processes.rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].process.started_at, STARTED_AT + 9_999);
}

#[test]
fn a_process_that_replaces_itself_takes_the_new_command_and_stays_open() {
    let fixture = fixture();
    start_helper(&fixture, HELPER_PID, "sh -c exec node");
    fixture.office.probe.set_descendants(&[HELPER_PID]);
    fixture.recorder.scan();

    start_helper(&fixture, HELPER_PID, "node server.js");
    fixture.recorder.scan();

    let rows = fixture.office.processes.rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].process.command_display, "node server.js");
}

#[test]
fn the_cli_row_is_never_dropped_by_a_scan() {
    let fixture = fixture();
    fixture.recorder.record_cli(&fixture.session);

    fixture
        .office
        .probe
        .set_read(CLI_PID, Err(ProbeError::NotRunning));
    fixture.recorder.scan();

    assert_eq!(fixture.office.processes.rows().len(), 1);
}

#[test]
fn a_session_stores_at_most_the_helper_limit_and_writes_one_entry() {
    let fixture = fixture();
    let helpers: Vec<u32> = (1_000..1_000 + 500).collect();
    for pid in &helpers {
        start_helper(&fixture, *pid, "sleep 1000");
    }
    fixture.office.probe.set_descendants(&helpers);

    fixture.recorder.scan();
    fixture.recorder.scan();

    assert_eq!(
        fixture.office.processes.rows().len(),
        MAX_OPEN_HELPERS_PER_SESSION
    );
    assert_eq!(fixture.office.processes.limited(), vec![fixture.session.id]);
    let entries = fixture.office.recorder.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, AgentAction::HelperLimit);
    assert_eq!(entries[0].actor, Actor::Office);
    assert!(
        entries[0]
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("200"))
    );
}

#[test]
fn a_session_that_ended_is_not_scanned() {
    let fixture = fixture();
    start_helper(&fixture, HELPER_PID, "node server.js");
    fixture.office.probe.set_descendants(&[HELPER_PID]);
    fixture
        .office
        .sessions
        .end_running(
            fixture.session.agent_id,
            &agent_domain::value_objects::ExitResult::Signal,
            Timestamp::from_unix_millis(2),
        )
        .expect("end");

    fixture.recorder.scan();

    assert_eq!(fixture.office.processes.rows(), Vec::new());
}

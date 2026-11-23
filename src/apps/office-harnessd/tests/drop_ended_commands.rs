// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]
#![cfg(unix)]

//! The evidence of 0023 against real server processes: after the next server
//! start, a byte search of the database file and its `-wal` and `-shm` files
//! finds no command line that left the store (SC-001 to SC-003, SC-007,
//! SC-008), and after a resume no ended identity of a `lost` session remains
//! while its sweep entries keep their text (SC-004, SC-005). The fake CLIs are
//! `sh` scripts. Each marker sits in the command line of a helper only, and
//! each byte search first proves that the marker was stored and readable in
//! the files, so a zero is never a marker that never got stored.

mod support;

use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use agent_domain::value_objects::SessionId;
use office_harnessd::config::Config;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use support::config_in;
use support::fake_cli::FakeClis;
use support::office_client::{Client, wait_for};
use support::server_process::{ServerProcess, connect, spawn_server, wait_until_started};

const STOPPED_BY_PERSON: usize = 10;
const ENDED_BY_CLI: usize = 5;
const ENDED_BY_CLEAN_STOP: usize = 5;
const SESSIONS_WITH_MARKER: usize = STOPPED_BY_PERSON + ENDED_BY_CLI + ENDED_BY_CLEAN_STOP;
const SESSIONS_WITH_ENDING_HELPER: usize = 10;
const SESSIONS_WITH_EXEC: usize = 10;
/// A helper that ends this long after it starts (SC-007).
const HELPER_LIFETIME_SECONDS: u32 = 15;
/// The wait before the stored process replaces itself. It is longer than the
/// scan period, so the old command line is stored before it is replaced.
const EXEC_DELAY_SECONDS: u32 = 12;
/// How long a stand-in process lives when nothing ends it, so a failed test
/// leaves it for two minutes only.
const LEFTOVER_SECONDS: u32 = 120;
const SLOW_SECONDS: u32 = 600;

const SWEEP_BOUND: Duration = Duration::from_secs(30);
const POLL: Duration = Duration::from_millis(50);
const DATABASE_SUFFIX: &str = "";
const WRITE_LOG_SUFFIX: &str = "-wal";
const SHARED_MEMORY_SUFFIX: &str = "-shm";
const FILE_SUFFIXES: [&str; 3] = [DATABASE_SUFFIX, WRITE_LOG_SUFFIX, SHARED_MEMORY_SUFFIX];

const PROCESS_STATE_FIELD_AFTER_NAME: usize = 0;
const START_TIME_FIELD_AFTER_NAME: usize = 19;
const ZOMBIE_STATE: &str = "Z";

const EMPTY_ADDITIONS: &str = r#"{"flags":[],"env":[]}"#;
const NORMAL_END: &str = "0";
const MARK_TASK: &str = "freed_text_cleared";
const ROW_PADDING: usize = 300;
const SWEEP_ENTRY_ACTION: &str = "end_leftover";

/// The unique text that only a helper command line carries.
fn marker(label: &str, index: usize) -> String {
    format!("MARK-{label}-{index:02}")
}

fn markers(label: &str, count: usize) -> Vec<String> {
    (0..count).map(|index| marker(label, index)).collect()
}

fn office_file(config: &Config, suffix: &str) -> PathBuf {
    let mut name = config.database_path().into_os_string();
    name.push(suffix);
    PathBuf::from(name)
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

/// The folders, the stand-in CLIs, and the server of one test.
struct Fixture {
    _folder: tempfile::TempDir,
    _work: tempfile::TempDir,
    records: tempfile::TempDir,
    work_path: PathBuf,
    config: Config,
    clis: FakeClis,
}

/// One started agent and its session.
struct Started {
    agent: String,
    session: String,
}

impl Fixture {
    fn new() -> Self {
        let folder = tempfile::tempdir().expect("folder");
        let work = tempfile::tempdir().expect("work folder");
        let clis = FakeClis::new();
        let _ = clis.login_setup();
        Self {
            work_path: work.path().to_path_buf(),
            config: config_in(&folder),
            _folder: folder,
            _work: work,
            records: tempfile::tempdir().expect("records"),
            clis,
        }
    }

    fn records_path(&self) -> &Path {
        self.records.path()
    }

    fn spawn(&self) -> ServerProcess {
        spawn_server(&self.config, &self.clis)
    }

    async fn client(&self) -> Client {
        connect(&self.config).await
    }

    fn database(&self) -> Connection {
        Connection::open(self.config.database_path()).expect("database")
    }

    async fn create(&self, client: &mut Client, name: &str, command: &str) -> String {
        let created = client
            .ask(
                "agent.create",
                json!({"name": name, "command": command, "working_folder": self.work_path}),
            )
            .await;
        assert_eq!(created["ok"], true, "{created}");
        created["result"]["id"].as_str().expect("id").to_owned()
    }

    async fn create_and_start(&self, client: &mut Client, name: &str, command: &str) -> Started {
        let agent = self.create(client, name, command).await;
        let session = client.start(&agent).await["id"]
            .as_str()
            .expect("session id")
            .to_owned();
        Started { agent, session }
    }

    /// Adds one stand-in CLI for each marker, named `{label}-cli-NN`, whose
    /// body `body_of` builds, and starts an agent on each.
    async fn start_sessions(
        &self,
        client: &mut Client,
        label: &str,
        markers: &[String],
        body_of: impl Fn(usize, &str, &str) -> String,
    ) -> Vec<Started> {
        let mut started = Vec::new();
        for (index, marker) in markers.iter().enumerate() {
            let command = format!("{label}-cli-{index:02}");
            self.clis.add(&command, &body_of(index, &command, marker));
            started.push(
                self.create_and_start(client, &format!("Agent {index:02}"), &command)
                    .await,
            );
        }
        started
    }

    fn identity_count(&self, session: &str) -> i64 {
        self.database()
            .query_row(
                "SELECT COUNT(*) FROM agent_session_processes WHERE session_id = ?1",
                [session],
                |row| row.get(0),
            )
            .expect("count")
    }

    fn session_has_ended(&self, session: &str) -> bool {
        self.database()
            .query_row(
                "SELECT ended_at IS NOT NULL FROM agent_sessions WHERE id = ?1",
                [session],
                |row| row.get(0),
            )
            .expect("session")
    }

    fn is_stored(&self, marker: &str) -> bool {
        let stored: i64 = self
            .database()
            .query_row(
                "SELECT COUNT(*) FROM agent_session_processes WHERE instr(command_display, ?1) > 0",
                [marker],
                |row| row.get(0),
            )
            .expect("count");
        stored > 0
    }

    /// The bytes of the database file and of its `-wal` and `-shm` files, as
    /// far as they exist.
    fn office_bytes(&self) -> Vec<Vec<u8>> {
        let files: Vec<Vec<u8>> = FILE_SUFFIXES
            .iter()
            .filter_map(|suffix| match fs::read(office_file(&self.config, suffix)) {
                Ok(bytes) => Some(bytes),
                Err(error) if error.kind() == ErrorKind::NotFound => None,
                Err(error) => panic!("an office file cannot be read: {error}"),
            })
            .collect();
        assert!(!files.is_empty(), "the database file exists");
        files
    }

    fn markers_in_files<'a>(&self, markers: &'a [String]) -> Vec<&'a String> {
        let files = self.office_bytes();
        markers
            .iter()
            .filter(|marker| files.iter().any(|bytes| contains(bytes, marker)))
            .collect()
    }

    /// The honesty check of every scan: waits until each marker is stored in
    /// a process identity and also found in the bytes of the files, so that a
    /// later zero cannot come from a marker that never got stored.
    async fn wait_until_stored_and_readable(&self, markers: &[String]) {
        let mut confirmed = BTreeSet::new();
        wait_for("every marker in the store and in the files", || {
            let files = self.office_bytes();
            for marker in markers {
                let in_files = files.iter().any(|bytes| contains(bytes, marker));
                if in_files && self.is_stored(marker) {
                    confirmed.insert(marker.clone());
                }
            }
            (confirmed.len() == markers.len()).then_some(())
        })
        .await;
    }

    fn assert_no_marker_in_files(&self, markers: &[String]) {
        let found = self.markers_in_files(markers);
        assert!(
            found.is_empty(),
            "{} of {} markers are still in the files: {found:?}",
            found.len(),
            markers.len()
        );
    }
}

/// Asks for a clean stop and waits until the server process has exited.
async fn stop_cleanly(client: &mut Client, server: &mut ServerProcess) {
    let answer = client.ask("server.restart", json!({})).await;
    assert_eq!(answer["ok"], true, "{answer}");
    wait_for("the server to exit", || server.0.try_wait().expect("poll")).await;
}

/// A CLI that starts one helper with `marker` in its command line, and then
/// runs `tail`.
fn helper_cli_body(helper_script: &str, tail: &str) -> String {
    format!("sh -c '{helper_script}' &\necho ready\n{tail}")
}

fn long_helper(marker: &str) -> String {
    format!("sleep {SLOW_SECONDS}; : {marker}")
}

#[tokio::test]
async fn twenty_sessions_that_end_normally_leave_no_identity_and_no_marker_after_the_next_start() {
    let fixture = Fixture::new();
    let mut server = fixture.spawn();
    wait_until_started(&fixture.config, 1).await;
    let mut client = fixture.client().await;
    let markers = markers("sc001", SESSIONS_WITH_MARKER);
    let records = fixture.records_path().to_path_buf();
    let sessions = fixture
        .start_sessions(&mut client, "sc001", &markers, |index, command, marker| {
            let tail = if (STOPPED_BY_PERSON..STOPPED_BY_PERSON + ENDED_BY_CLI).contains(&index) {
                format!(
                    "while [ ! -e {}/end-{command} ]; do sleep 1; done",
                    records.display()
                )
            } else {
                "wait".to_owned()
            };
            helper_cli_body(&long_helper(marker), &tail)
        })
        .await;
    fixture.wait_until_stored_and_readable(&markers).await;
    let (stopped, rest) = sessions.split_at(STOPPED_BY_PERSON);
    let (ending_by_cli, clean_stops) = rest.split_at(ENDED_BY_CLI);

    for session in stopped {
        let answer = client
            .ask("agent.stop", json!({"agent_id": session.agent}))
            .await;
        assert_eq!(answer["ok"], true, "{answer}");
        assert_eq!(fixture.identity_count(&session.session), 0);
    }
    for (offset, session) in ending_by_cli.iter().enumerate() {
        let command = format!("sc001-cli-{:02}", STOPPED_BY_PERSON + offset);
        fs::write(records.join(format!("end-{command}")), "").expect("gate");
        wait_for("the CLI to end its session", || {
            fixture.session_has_ended(&session.session).then_some(())
        })
        .await;
        assert_eq!(fixture.identity_count(&session.session), 0);
    }
    stop_cleanly(&mut client, &mut server).await;
    for session in clean_stops {
        assert!(fixture.session_has_ended(&session.session));
    }

    for session in &sessions {
        assert_eq!(fixture.identity_count(&session.session), 0);
    }
    let _second = fixture.spawn();
    wait_until_started(&fixture.config, 2).await;
    fixture.assert_no_marker_in_files(&markers);
}

#[tokio::test]
async fn ten_helpers_that_end_while_their_session_runs_leave_no_marker_after_the_next_start() {
    let fixture = Fixture::new();
    let mut server = fixture.spawn();
    wait_until_started(&fixture.config, 1).await;
    let mut client = fixture.client().await;
    let markers = markers("sc007", SESSIONS_WITH_ENDING_HELPER);
    let sessions = fixture
        .start_sessions(&mut client, "sc007", &markers, |_, _, marker| {
            helper_cli_body(
                &format!("sleep {HELPER_LIFETIME_SECONDS}; : {marker}"),
                &format!("sleep {SLOW_SECONDS}"),
            )
        })
        .await;
    fixture.wait_until_stored_and_readable(&markers).await;

    wait_for("every helper to end and be dropped", || {
        markers
            .iter()
            .all(|marker| !fixture.is_stored(marker))
            .then_some(())
    })
    .await;

    for session in &sessions {
        assert!(!fixture.session_has_ended(&session.session));
    }
    stop_cleanly(&mut client, &mut server).await;
    let _second = fixture.spawn();
    wait_until_started(&fixture.config, 2).await;
    fixture.assert_no_marker_in_files(&markers);
}

#[tokio::test]
async fn ten_replaced_command_lines_leave_no_old_marker_after_the_next_start() {
    let fixture = Fixture::new();
    let mut server = fixture.spawn();
    wait_until_started(&fixture.config, 1).await;
    let mut client = fixture.client().await;
    let markers = markers("sc008", SESSIONS_WITH_EXEC);
    let records = fixture.records_path().to_path_buf();
    let sessions = fixture
        .start_sessions(&mut client, "sc008", &markers, |_, command, marker| {
            helper_cli_body(
                &format!(
                    "printf \"%s\" \"$$\" > {}/pid-{command}; : {marker}; sleep {EXEC_DELAY_SECONDS}; exec sleep {SLOW_SECONDS}",
                    records.display()
                ),
                "wait",
            )
        })
        .await;
    fixture.wait_until_stored_and_readable(&markers).await;

    for (index, session) in sessions.iter().enumerate() {
        let pid_file = records.join(format!("pid-sc008-cli-{index:02}"));
        let pid: i64 = fs::read_to_string(pid_file)
            .expect("the pid of the replacing process")
            .trim()
            .parse()
            .expect("a pid");
        wait_for("the replaced command line", || {
            let stored: Option<String> = fixture
                .database()
                .query_row(
                    "SELECT command_display FROM agent_session_processes \
                     WHERE session_id = ?1 AND pid = ?2",
                    params![session.session, pid],
                    |row| row.get(0),
                )
                .ok();
            stored
                .filter(|display| {
                    display.starts_with("sleep") && !display.contains(&markers[index])
                })
                .map(|_| ())
        })
        .await;
    }

    stop_cleanly(&mut client, &mut server).await;
    let _second = fixture.spawn();
    wait_until_started(&fixture.config, 2).await;
    fixture.assert_no_marker_in_files(&markers);
}

fn insert_session(connection: &Connection, agent: &str, started_at: i64) -> String {
    let session_id = SessionId::generate().to_string();
    connection
        .execute(
            "INSERT INTO agent_sessions \
             (id, agent_id, additions_snapshot, started_at, ended_at, exit_result) \
             VALUES (?1, ?2, ?3, ?4, ?4, ?5)",
            params![session_id, agent, EMPTY_ADDITIONS, started_at, NORMAL_END],
        )
        .expect("session");
    session_id
}

/// A row as the 0017 build stored it: a long text with the marker first.
fn insert_identity(connection: &Connection, session: &str, pid: i64, marker: &str, state: &str) {
    connection
        .execute(
            "INSERT INTO agent_session_processes \
             (session_id, pid, started_at, command_hash, command_display, boot, kind, state) \
             VALUES (?1, ?2, 1, zeroblob(32), ?3, 'boot', 'helper', ?4)",
            params![
                session,
                pid,
                format!("{marker} {}", "x".repeat(ROW_PADDING)),
                state
            ],
        )
        .expect("identity");
}

/// What the 0017 build left: secure delete off, 10 sessions that ended with
/// a marker in an ended identity, and in one more session 10 helper
/// identities that were dropped and 10 command lines that were replaced.
/// Writes, drops, replaces and ends identities as the 0017 build did, with
/// secure delete off, and gives back the open connection, so that the caller
/// chooses how that build stopped.
fn write_as_the_0017_build(config: &Config, agent: &str) -> (Connection, Vec<String>) {
    let connection = Connection::open(config.database_path()).expect("database");
    connection
        .execute_batch("PRAGMA secure_delete = OFF; PRAGMA wal_autocheckpoint = 0")
        .expect("secure delete off");
    connection
        .execute("DELETE FROM agent_maintenance WHERE task = ?1", [MARK_TASK])
        .expect("the 0017 build had no mark");
    let ended = markers("sc003-ended", 10);
    for (index, marker) in ended.iter().enumerate() {
        let started_at = i64::try_from(index).expect("index") + 100;
        let session = insert_session(&connection, agent, started_at);
        insert_identity(&connection, &session, 1, marker, "ended");
    }
    let holder = insert_session(&connection, agent, 200);
    let dropped = markers("sc003-dropped", 10);
    for (pid, marker) in (10_i64..).zip(&dropped) {
        insert_identity(&connection, &holder, pid, marker, "open");
        connection
            .execute(
                "DELETE FROM agent_session_processes WHERE session_id = ?1 AND pid = ?2",
                params![holder, pid],
            )
            .expect("dropped helper");
    }
    let replaced = markers("sc003-replaced", 10);
    for (pid, marker) in (30_i64..).zip(&replaced) {
        insert_identity(&connection, &holder, pid, marker, "open");
        connection
            .execute(
                "UPDATE agent_session_processes SET command_display = 'sleep 600' \
                 WHERE session_id = ?1 AND pid = ?2",
                params![holder, pid],
            )
            .expect("replaced command line");
    }
    connection
        .execute(
            "UPDATE agent_session_processes SET state = 'ended' WHERE session_id = ?1",
            [&holder],
        )
        .expect("the session ended");
    (connection, [ended, dropped, replaced].concat())
}

/// A server that stops without closing its connection leaves its write log
/// with the frames of every commit. Closing the connection here would
/// checkpoint and delete that log, so the files are copied first and put back
/// after the close.
fn leave_the_files_as_a_stopped_build_does(config: &Config, connection: Connection) {
    let image = tempfile::tempdir().expect("image folder");
    for suffix in [DATABASE_SUFFIX, WRITE_LOG_SUFFIX] {
        fs::copy(
            office_file(config, suffix),
            image.path().join(format!("office{suffix}")),
        )
        .expect("copy the file");
    }
    drop(connection);
    let _ = fs::remove_file(office_file(config, SHARED_MEMORY_SUFFIX));
    for suffix in [DATABASE_SUFFIX, WRITE_LOG_SUFFIX] {
        fs::copy(
            image.path().join(format!("office{suffix}")),
            office_file(config, suffix),
        )
        .expect("put the file back");
    }
}

#[tokio::test]
async fn a_database_of_the_0017_build_holds_no_marker_after_its_first_start_of_this_build() {
    let fixture = Fixture::new();
    let agent = fixture.agent_of_a_stopped_server().await;
    let (connection, markers) = write_as_the_0017_build(&fixture.config, &agent);
    leave_the_files_as_a_stopped_build_does(&fixture.config, connection);
    let before = fixture.markers_in_files(&markers);
    assert_eq!(
        before.len(),
        markers.len(),
        "the 0017 build left every marker in the files"
    );

    fixture.start_this_build_and_find_no_marker(&markers).await;
}

/// A clean stop checkpoints the write log into the database file, so the
/// freed text sits in the file's free cells and pages instead of the log.
#[tokio::test]
async fn a_cleanly_stopped_database_of_the_0017_build_holds_no_marker_after_its_first_start() {
    let fixture = Fixture::new();
    let agent = fixture.agent_of_a_stopped_server().await;
    let (connection, markers) = write_as_the_0017_build(&fixture.config, &agent);
    drop(connection);
    let before = fixture.markers_in_files(&markers);
    assert!(!before.is_empty(), "the clean stop left no marker to clear");

    fixture.start_this_build_and_find_no_marker(&markers).await;
}

impl Fixture {
    /// A first start of this build creates the schema and one agent; the
    /// server then stops cleanly.
    async fn agent_of_a_stopped_server(&self) -> String {
        let mut first = self.spawn();
        wait_until_started(&self.config, 1).await;
        let mut client = self.client().await;
        let agent = self.create(&mut client, "Ana", "claude").await;
        stop_cleanly(&mut client, &mut first).await;
        agent
    }

    async fn start_this_build_and_find_no_marker(&self, markers: &[String]) {
        let _second = self.spawn();
        wait_until_started(&self.config, 2).await;

        self.assert_no_marker_in_files(markers);
        let identities: i64 = self
            .database()
            .query_row("SELECT COUNT(*) FROM agent_session_processes", [], |row| {
                row.get(0)
            })
            .expect("count");
        assert_eq!(identities, 0);
        let ended_by_the_sweep: i64 = self
            .database()
            .query_row(
                "SELECT COUNT(*) FROM audit_actions WHERE action = ?1",
                [SWEEP_ENTRY_ACTION],
                |row| row.get(0),
            )
            .expect("count");
        assert_eq!(ended_by_the_sweep, 0);
    }
}

/// One process as the test saw it: the start time tells a reused process id
/// apart.
fn start_time_of(pid: u32) -> i64 {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).expect("stat");
    let after_name = stat.rsplit_once(')').expect("a name").1;
    let fields: Vec<&str> = after_name.split_whitespace().collect();
    assert_ne!(fields[PROCESS_STATE_FIELD_AFTER_NAME], ZOMBIE_STATE);
    fields[START_TIME_FIELD_AFTER_NAME]
        .parse()
        .expect("a start time")
}

/// A process the office never saw, with no command line, that the sweep
/// leaves running. It ends with the test.
struct Decoy(Child);

impl Drop for Decoy {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A CLI that survives the server's death, with a helper in a session of its
/// own, and that only sleeps when the office runs it again.
fn leftover_body(records: &Path, command: &str) -> String {
    let ran = format!("{}/ran-{command}", records.display());
    format!(
        "if [ -e {ran} ]; then echo ready; exec sleep {LEFTOVER_SECONDS}; fi\n: > {ran}\ntrap '' HUP\nsetsid sh -c 'exec sleep {LEFTOVER_SECONDS}' &\necho ready\nwait"
    )
}

/// A CLI that ends with the hang-up of the server's death.
fn hung_up_body(records: &Path, command: &str) -> String {
    let ran = format!("{}/ran-{command}", records.display());
    format!(
        "if [ -e {ran} ]; then echo ready; exec sleep {LEFTOVER_SECONDS}; fi\n: > {ran}\necho ready\nexec sleep {LEFTOVER_SECONDS}"
    )
}

async fn plan_after_the_sweep(client: &mut Client) -> Value {
    let deadline = Instant::now() + SWEEP_BOUND;
    loop {
        let plan = client.ask("resume.plan", json!({})).await;
        assert_eq!(plan["ok"], true, "{plan}");
        if plan["result"]["sweeping"] == false {
            return plan["result"].clone();
        }
        assert!(
            Instant::now() < deadline,
            "the sweep took over {SWEEP_BOUND:?}"
        );
        tokio::time::sleep(POLL).await;
    }
}

fn assert_marks_of_the_plan(plan: &Value, expected: &[(String, Value, bool)]) {
    for (agent, finding, warns) in expected {
        let entry = plan["agents"]
            .as_array()
            .expect("agents")
            .iter()
            .find(|entry| entry["agent_id"] == *agent)
            .unwrap_or_else(|| panic!("no entry for {agent}: {plan}"));
        assert_eq!(entry["unclean_stop"], *finding, "{entry}");
        assert_eq!(entry["after_unclean_stop"], *warns, "{entry}");
    }
}

/// Actor, agent, result, and detail of one sweep entry.
type SweepEntry = (String, Option<String>, String, String);

fn sweep_entries(fixture: &Fixture) -> Vec<SweepEntry> {
    let database = fixture.database();
    let mut statement = database
        .prepare(
            "SELECT actor, target_agent_id, result, detail FROM audit_actions \
             WHERE action = ?1 ORDER BY id",
        )
        .expect("prepare");
    statement
        .query_map([SWEEP_ENTRY_ACTION], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("entries")
}

fn open_identities(fixture: &Fixture, session: &str) -> i64 {
    fixture
        .database()
        .query_row(
            "SELECT COUNT(*) FROM agent_session_processes WHERE state = 'open' AND session_id = ?1",
            [session],
            |row| row.get(0),
        )
        .expect("count")
}

fn kill_server(server: &mut ServerProcess) {
    server.0.kill().expect("SIGKILL the server");
    server.0.wait().expect("reap the server");
}

/// The four agents of SC-004: a leftover that the sweep ends, one that it
/// cannot read, a CLI that ended on the hang-up, and a session with no stored
/// identity (one from before 0017).
struct LostAgents {
    ended: Started,
    unreadable: Started,
    hung_up: Started,
    no_identity: Started,
}

impl LostAgents {
    async fn start(fixture: &Fixture, client: &mut Client) -> Self {
        let records = fixture.records_path();
        for (command, body) in [
            ("ended-cli", leftover_body(records, "ended-cli")),
            ("unreadable-cli", leftover_body(records, "unreadable-cli")),
            ("hung-up-cli", hung_up_body(records, "hung-up-cli")),
            ("no-identity-cli", hung_up_body(records, "no-identity-cli")),
        ] {
            fixture.clis.add(command, &body);
        }
        let agents = Self {
            ended: fixture.create_and_start(client, "Ended", "ended-cli").await,
            unreadable: fixture
                .create_and_start(client, "Unreadable", "unreadable-cli")
                .await,
            hung_up: fixture
                .create_and_start(client, "Hangs up", "hung-up-cli")
                .await,
            no_identity: fixture
                .create_and_start(client, "No identity", "no-identity-cli")
                .await,
        };
        for (started, count) in [
            (&agents.ended, 2),
            (&agents.unreadable, 2),
            (&agents.hung_up, 1),
            (&agents.no_identity, 1),
        ] {
            wait_for("the stored identities of the stand-ins", || {
                (open_identities(fixture, &started.session) >= count).then_some(())
            })
            .await;
        }
        agents
    }

    fn all(&self) -> [&Started; 4] {
        [
            &self.ended,
            &self.unreadable,
            &self.hung_up,
            &self.no_identity,
        ]
    }

    fn sessions(&self) -> Vec<&str> {
        self.all()
            .iter()
            .map(|started| started.session.as_str())
            .collect()
    }

    fn agent_ids(&self) -> Vec<String> {
        self.all()
            .iter()
            .map(|started| started.agent.clone())
            .collect()
    }

    /// Stores an open identity on a process the office never saw, which the
    /// sweep leaves running, and deletes every identity of the session from
    /// before 0017. Runs while the server is dead.
    fn prepare_the_unclean_stop(&self, fixture: &Fixture) -> Decoy {
        let decoy = Decoy(
            Command::new("bash")
                .arg0("")
                .stdin(Stdio::piped())
                .spawn()
                .expect("a process without a command line"),
        );
        let database = fixture.database();
        database
            .execute(
                "INSERT INTO agent_session_processes \
                 (session_id, pid, started_at, command_hash, command_display, boot, kind, state) \
                 VALUES (?1, ?2, ?3, zeroblob(32), 'unread', 'boot', 'helper', 'open')",
                params![
                    self.unreadable.session,
                    decoy.0.id(),
                    start_time_of(decoy.0.id())
                ],
            )
            .expect("an unreadable identity");
        database
            .execute(
                "DELETE FROM agent_session_processes WHERE session_id = ?1",
                [&self.no_identity.session],
            )
            .expect("no identity");
        decoy
    }

    /// The mark that the plan gives each agent (0017/SC-005).
    fn expected_marks(&self) -> Vec<(String, Value, bool)> {
        vec![
            (self.ended.agent.clone(), json!({"ended": 2}), false),
            (
                self.unreadable.agent.clone(),
                json!({"may_run": "not readable"}),
                true,
            ),
            (self.hung_up.agent.clone(), Value::Null, false),
            (
                self.no_identity.agent.clone(),
                json!({"may_run": "no stored process"}),
                true,
            ),
        ]
    }
}

fn open_sessions(fixture: &Fixture) -> usize {
    let open: i64 = fixture
        .database()
        .query_row(
            "SELECT COUNT(*) FROM agent_sessions WHERE ended_at IS NULL",
            [],
            |row| row.get(0),
        )
        .expect("count");
    usize::try_from(open).expect("a count")
}

#[tokio::test]
async fn after_the_resume_no_ended_identity_of_the_lost_sessions_remains_and_the_sweep_entries_keep_their_text()
 {
    let fixture = Fixture::new();
    let mut server = fixture.spawn();
    wait_until_started(&fixture.config, 1).await;
    let mut client = fixture.client().await;
    let lost = LostAgents::start(&fixture, &mut client).await;
    kill_server(&mut server);
    drop(client);
    let _decoy = lost.prepare_the_unclean_stop(&fixture);
    let expected_marks = lost.expected_marks();

    let mut second = fixture.spawn();
    wait_until_started(&fixture.config, 2).await;
    let mut client = fixture.client().await;
    assert_marks_of_the_plan(&plan_after_the_sweep(&mut client).await, &expected_marks);
    kill_server(&mut second);
    drop(client);
    let _third = fixture.spawn();
    wait_until_started(&fixture.config, 3).await;
    let mut client = fixture.client().await;
    assert_marks_of_the_plan(&plan_after_the_sweep(&mut client).await, &expected_marks);

    let identities_before: i64 = lost
        .sessions()
        .iter()
        .map(|session| fixture.identity_count(session))
        .sum();
    assert!(
        identities_before > 0,
        "the last lost sessions keep their identities"
    );
    let entries_before = sweep_entries(&fixture);
    assert_ne!(entries_before.len(), 0, "the sweep wrote entries");
    let agents = lost.agent_ids();
    client
        .send("resume.start", json!({"agent_ids": agents}))
        .await;
    wait_for("every agent to store a new session", || {
        (open_sessions(&fixture) >= agents.len()).then_some(())
    })
    .await;

    for session in lost.sessions() {
        assert_eq!(fixture.identity_count(session), 0, "{session}");
    }
    assert_eq!(sweep_entries(&fixture), entries_before);
    for agent in &agents {
        client.ask("agent.stop", json!({"agent_id": agent})).await;
    }
}

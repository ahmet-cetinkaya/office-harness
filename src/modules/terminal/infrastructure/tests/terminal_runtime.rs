// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! Runs the real runtime against the fixture child in a PTY. Tests that need
//! raw keyboard input are Unix only: the fixture cannot set Windows console
//! modes without `unsafe` code.

mod support;

use std::time::Duration;

use kernel::AgentId;
use support::{Client, limits, runtime_in, size, spawn_fixture, wait_until};
#[cfg(unix)]
use support::{process_is_alive, spawn_program};
use tempfile::TempDir;
use terminal_application::common::interfaces::TerminalRuntime;
use terminal_domain::events::TerminalEvent;
use terminal_infrastructure::services::OutputStore;

const GRACE: Duration = Duration::from_millis(300);

#[cfg(unix)]
#[test]
fn keys_reach_the_child_unchanged() {
    let folder = TempDir::new().expect("temp folder");
    let (runtime, sink) = runtime_in(folder.path(), limits(1000));
    let agent = AgentId::generate();
    spawn_fixture(&runtime, agent, &["keys"]);
    let client = Client::attach(&runtime, agent, size(80, 24));
    wait_until("the child to be ready", || client.text().contains("READY"));

    for keys in [&b"\x03"[..], b"\x1b[A", b"\x1b[B", b"\r"] {
        runtime.write_input(agent, keys).expect("write");
    }
    wait_until("every key to arrive", || {
        let text = client.text();
        ["KEY:03", "KEY:1b", "KEY:5b", "KEY:41", "KEY:42", "KEY:0d"]
            .iter()
            .all(|key| text.contains(key))
    });
    runtime.write_input(agent, b"q").expect("write");
    wait_until("the exit", || sink.has_exited());
}

#[test]
fn an_attach_after_a_full_screen_draw_redraws_that_view() {
    let folder = TempDir::new().expect("temp folder");
    let (runtime, _sink) = runtime_in(folder.path(), limits(1000));
    let agent = AgentId::generate();
    spawn_fixture(&runtime, agent, &["altscreen"]);
    wait_until("the draw to be recorded", || {
        let probe = Client::attach(&runtime, agent, size(80, 24));
        let drawn = probe.text().contains("FULLSCREEN-MARKER");
        runtime.detach(agent, probe.subscription.expect("running terminal"));
        drawn
    });

    let late = Client::attach(&runtime, agent, size(80, 24));
    let text = late.text();
    assert!(text.contains("FULLSCREEN-MARKER"));
    assert!(text.contains("\x1b[?1049h"), "the alternate screen is on");
    runtime.stop(agent, GRACE).expect("stop");
}

#[test]
fn output_that_arrives_during_the_attach_appears_exactly_once() {
    let folder = TempDir::new().expect("temp folder");
    let (runtime, sink) = runtime_in(folder.path(), limits(10_000));
    let agent = AgentId::generate();
    spawn_fixture(&runtime, agent, &["counter", "300", "3"]);
    std::thread::sleep(Duration::from_millis(150));
    let client = Client::attach(&runtime, agent, size(80, 24));
    wait_until("the last line", || client.text().contains("DONE"));

    let text = client.text();
    for line in 0..300 {
        let token = format!("L{line:03}.");
        assert_eq!(text.matches(&token).count(), 1, "{token}");
    }
    wait_until("the exit", || sink.has_exited());
}

#[cfg(unix)]
#[test]
fn a_resize_reaches_the_child_and_a_repeat_sends_nothing() {
    let folder = TempDir::new().expect("temp folder");
    let (runtime, _sink) = runtime_in(folder.path(), limits(1000));
    let agent = AgentId::generate();
    spawn_fixture(&runtime, agent, &["size"]);
    let client = Client::attach(&runtime, agent, size(80, 24));
    wait_until("the first size", || client.text().contains("SIZE:24 80"));

    assert!(runtime.resize(agent, size(100, 30)).expect("resize"));
    wait_until("the new size", || client.text().contains("SIZE:30 100"));
    assert!(!runtime.resize(agent, size(100, 30)).expect("resize"));
    runtime.stop(agent, GRACE).expect("stop");
}

#[test]
fn output_is_recorded_without_a_client_and_survives_a_server_restart() {
    let folder = TempDir::new().expect("temp folder");
    let agent = AgentId::generate();
    let saved_key = {
        let (runtime, sink) = runtime_in(folder.path(), limits(1000));
        let key = spawn_fixture(&runtime, agent, &["counter", "50", "2"]);
        wait_until("the exit", || sink.has_exited());
        key
    };

    let (restarted, _sink) = runtime_in(folder.path(), limits(1000));
    let attachment = restarted
        .attach(
            agent,
            Some(&saved_key),
            size(80, 24),
            std::sync::Arc::new(support_null()),
        )
        .expect("attach saved output");
    assert!(attachment.subscription.is_none());
    let text = String::from_utf8_lossy(&attachment.snapshot);
    assert!(text.contains("DONE") && text.contains("L049."));
}

/// `ConPTY` writes screen or cursor sequences when the child exits, and that
/// counts as output after the quiet period. It is real platform behavior, so
/// Windows accepts one `OutputResumed` directly before `Exited`.
fn without_exit_output_on_windows(mut events: Vec<TerminalEvent>) -> Vec<TerminalEvent> {
    let exit_position = events.len().saturating_sub(1);
    if cfg!(windows)
        && exit_position > 0
        && events[exit_position - 1] == TerminalEvent::OutputResumed
    {
        events.remove(exit_position - 1);
    }
    events
}

#[test]
fn events_follow_the_child() {
    let folder = TempDir::new().expect("temp folder");
    let (runtime, sink) = runtime_in(folder.path(), limits(1000));
    let agent = AgentId::generate();
    spawn_fixture(&runtime, agent, &["delayed-exit", "600", "7"]);
    wait_until("the exit", || sink.has_exited());
    assert_eq!(
        without_exit_output_on_windows(sink.events()),
        [
            TerminalEvent::FirstOutput,
            TerminalEvent::QuietPeriod,
            TerminalEvent::Exited { exit_code: 7 }
        ]
    );
}

#[test]
fn output_after_a_quiet_period_is_reported_once_for_the_change_not_for_each_chunk() {
    let folder = TempDir::new().expect("temp folder");
    let (runtime, sink) = runtime_in(folder.path(), limits(1000));
    let agent = AgentId::generate();
    spawn_fixture(&runtime, agent, &["pause-resume", "700"]);
    wait_until("the exit", || sink.has_exited());
    assert_eq!(
        without_exit_output_on_windows(sink.events()),
        [
            TerminalEvent::FirstOutput,
            TerminalEvent::QuietPeriod,
            TerminalEvent::OutputResumed,
            TerminalEvent::QuietPeriod,
            TerminalEvent::Exited { exit_code: 0 }
        ]
    );
}

#[cfg(unix)]
#[test]
fn stop_ends_a_child_that_ignores_the_request_and_its_grandchild() {
    let folder = TempDir::new().expect("temp folder");
    let (runtime, sink) = runtime_in(folder.path(), limits(1000));
    let agent = AgentId::generate();
    let script = "trap '' TERM; sleep 300 & echo GRANDCHILD:$!; while :; do sleep 1; done";
    spawn_program(&runtime, agent, "/bin/sh", &["-c", script]);
    let client = Client::attach(&runtime, agent, size(80, 24));
    wait_until("the grandchild id", || {
        client.text().contains("GRANDCHILD:")
    });
    let text = client.text();
    let digits: String = text
        .split("GRANDCHILD:")
        .nth(1)
        .expect("marker")
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    let grandchild: u32 = digits.parse().expect("process id");
    assert!(process_is_alive(grandchild));

    runtime.stop(agent, GRACE).expect("stop");
    wait_until("the exit event", || sink.has_exited());
    wait_until("the grandchild to end", || !process_is_alive(grandchild));
    assert!(!runtime.is_running(agent));
}

#[test]
fn output_above_the_limit_drops_the_oldest_output_and_reports_it() {
    let folder = TempDir::new().expect("temp folder");
    let (runtime, sink) = runtime_in(folder.path(), limits(50));
    let agent = AgentId::generate();
    let key = spawn_fixture(&runtime, agent, &["counter", "300", "2"]);
    let client = Client::attach(&runtime, agent, size(80, 24));
    wait_until("the exit", || sink.has_exited());

    assert!(client.dropped_notices() > 0);
    let saved = OutputStore::read_saved(folder.path(), &key)
        .expect("read")
        .expect("file exists");
    let saved = String::from_utf8_lossy(&saved);
    assert!(!saved.contains("L000."));
    assert!(saved.contains("L299."));
}

#[cfg(unix)]
#[test]
fn two_clients_receive_the_output_and_both_can_type() {
    let folder = TempDir::new().expect("temp folder");
    let (runtime, sink) = runtime_in(folder.path(), limits(1000));
    let agent = AgentId::generate();
    spawn_fixture(&runtime, agent, &["keys"]);
    let first = Client::attach(&runtime, agent, size(80, 24));
    let second = Client::attach(&runtime, agent, size(100, 30));
    wait_until("the child to be ready", || first.text().contains("READY"));

    runtime
        .write_input(agent, b"a")
        .expect("write from the first client");
    runtime
        .write_input(agent, b"b")
        .expect("write from the second client");
    wait_until("both keys on both clients", || {
        [&first, &second]
            .iter()
            .all(|client| client.text().contains("KEY:61") && client.text().contains("KEY:62"))
    });

    // The second attach resized to 100x30 last, so that size is the current one.
    assert!(!runtime.resize(agent, size(100, 30)).expect("resize"));
    assert!(runtime.resize(agent, size(80, 24)).expect("resize"));
    runtime.write_input(agent, b"q").expect("write");
    wait_until("the exit", || sink.has_exited());
}

#[cfg(unix)]
#[test]
fn a_cursor_query_is_answered_with_no_client_attached() {
    let folder = TempDir::new().expect("temp folder");
    let (runtime, sink) = runtime_in(folder.path(), limits(1000));
    let agent = AgentId::generate();
    let key = spawn_fixture(&runtime, agent, &["query"]);
    wait_until("the exit", || sink.has_exited());
    let saved = OutputStore::read_saved(folder.path(), &key)
        .expect("read")
        .expect("file exists");
    // The cursor stands at row 2, column 1 after the first line: ESC [ 2 ; 1 R.
    assert!(String::from_utf8_lossy(&saved).contains("GOT:1b5b323b3152"));
}

#[cfg(unix)]
#[test]
fn a_query_split_across_two_reads_gets_one_answer() {
    let folder = TempDir::new().expect("temp folder");
    let (runtime, sink) = runtime_in(folder.path(), limits(1000));
    let agent = AgentId::generate();
    let key = spawn_fixture(&runtime, agent, &["query-split"]);
    wait_until("the exit", || sink.has_exited());
    let saved = OutputStore::read_saved(folder.path(), &key)
        .expect("read")
        .expect("file exists");
    assert!(String::from_utf8_lossy(&saved).contains("REPLIES:1"));
}

fn support_null() -> impl terminal_application::common::interfaces::OutputSubscriber {
    struct Null;
    impl terminal_application::common::interfaces::OutputSubscriber for Null {
        fn deliver(
            &self,
            _output: terminal_application::common::models::LiveOutput,
        ) -> terminal_application::common::models::DeliveryStatus {
            terminal_application::common::models::DeliveryStatus::Closed
        }
    }
    Null
}

#[test]
fn a_command_that_cannot_start_keeps_the_saved_output_of_the_last_session() {
    let folder = TempDir::new().expect("temp folder");
    let (runtime, sink) = runtime_in(folder.path(), limits(1000));
    let agent = AgentId::generate();
    let key = spawn_fixture(&runtime, agent, &["counter", "20", "1"]);
    wait_until("the exit", || sink.has_exited());

    let missing = folder.path().join("no-such-program");
    let request = terminal_application::common::models::SpawnTerminalRequest {
        agent_id: agent,
        program: missing.to_string_lossy().into_owned(),
        arguments: Vec::new(),
        working_folder: std::env::temp_dir(),
        environment: Vec::new(),
        size: size(80, 24),
        output_key: key.clone(),
    };
    assert!(runtime.spawn(&request).is_err());

    let saved = OutputStore::read_saved(folder.path(), &key)
        .expect("read")
        .expect("the earlier output is still there");
    assert!(String::from_utf8_lossy(&saved).contains("DONE"));
}

#[test]
fn the_last_line_is_reported_when_it_changes_and_throttled_after() {
    let folder = TempDir::new().expect("temp folder");
    let (runtime, sink) = runtime_in(folder.path(), limits(1000));
    let agent = AgentId::generate();
    spawn_fixture(&runtime, agent, &["progress"]);
    // The monitor reports the screen as it stands on its first tick, then at
    // most once per second. The nine redraws all land inside that first
    // window, so the newest text arrives in the second report at the latest.
    wait_until("the last redraw to be reported", || {
        sink.lines().last() == Some(&"PROGRESS:9%".to_owned())
    });
    // The screen no longer changes, so a whole report interval later there
    // is still no further report.
    std::thread::sleep(Duration::from_millis(1100));
    let reports = sink.lines();
    assert_eq!(reports.last(), Some(&"PROGRESS:9%".to_owned()));
    assert!(
        reports.len() <= 2,
        "the throttle holds the unchanged screen back: {reports:?}"
    );
    runtime.stop(agent, GRACE).expect("stop");
}

#[test]
fn a_bell_from_the_child_becomes_one_bell_event() {
    const TAP: bool = true; // TEMPORARY DEBUG TAP - remove before merge
    let folder = TempDir::new().expect("temp folder");
    let (runtime, sink) = runtime_in(folder.path(), limits(1000));
    let agent = AgentId::generate();
    spawn_fixture(&runtime, agent, &["lines"]);
    let client = Client::attach(&runtime, agent, size(80, 24));
    runtime
        .write_input(agent, b"a")
        .expect("write to confirm the child reads");
    wait_until("the child to be reading", || !sink.lines().is_empty());
    runtime.write_input(agent, b"b").expect("ring the bell");
    let sleep_result = wait_until_tap(&sink);
    if TAP {
        let stream = client.raw();
        eprintln!("=== BELL TAP BEGIN ===");
        eprintln!("master stream after timeout: {} bytes", stream.len());
        eprintln!("hexdump of the last 400 bytes:");
        for chunk in stream.len().saturating_sub(400)..stream.len() {
            eprintln!("{:04x}: {:02x}", chunk, stream[chunk]);
        }
        eprintln!(
            "0x07 occurrences in master stream: {}",
            stream.iter().filter(|byte| **byte == 0x07).count()
        );
        eprintln!(
            "BELLED marker in master stream: {}",
            client.text().contains("BELLED")
        );
        eprintln!("events at timeout: {:?}", sink.events());
        eprintln!("lines at timeout: {:?}", sink.lines());
        eprintln!("=== BELL TAP END ===");
    }
    panic!("tap: dumping state instead of waiting ({sleep_result:?})");
}

fn wait_until_tap(sink: &support::RecordingSink) -> std::time::Duration {
    let started = std::time::Instant::now();
    while started.elapsed() < std::time::Duration::from_secs(12) {
        if sink
            .events()
            .iter()
            .any(|event| matches!(event, TerminalEvent::Bell))
        {
            panic!("the bell was reported; the tap is not needed on this platform");
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    started.elapsed()
}

#[test]
fn the_runtime_refuses_an_unsafe_batch_script_whoever_calls_it() {
    let folder = TempDir::new().expect("temp folder");
    let (runtime, _sink) = runtime_in(folder.path(), limits(1000));
    let agent = AgentId::generate();
    let request = terminal_application::common::models::SpawnTerminalRequest {
        agent_id: agent,
        program: "tool.cmd".to_owned(),
        arguments: vec!["50%".to_owned()],
        working_folder: std::env::temp_dir(),
        environment: Vec::new(),
        size: size(80, 24),
        output_key: terminal_domain::value_objects::OutputKey::parse("x").expect("key"),
    };
    assert!(matches!(
        runtime.spawn(&request),
        Err(terminal_application::common::errors::TerminalRuntimeError::Invalid(_))
    ));
}

#[test]
fn each_start_has_a_new_run_and_its_events_name_it() {
    let folder = TempDir::new().expect("temp folder");
    let (runtime, sink) = runtime_in(folder.path(), limits(1000));
    let agent = AgentId::generate();
    assert_eq!(runtime.current_run(agent), None);

    spawn_fixture(&runtime, agent, &["exit", "0"]);
    wait_until("the first exit", || sink.has_exited());
    let first = runtime.current_run(agent).expect("first run");
    assert!(sink.runs().iter().all(|run| *run == first));

    let events_before = sink.events().len();
    spawn_fixture(&runtime, agent, &["exit", "0"]);
    wait_until("the second exit", || sink.events().len() > events_before);
    let second = runtime.current_run(agent).expect("second run");

    assert!(second > first);
    assert!(
        sink.runs()[events_before..]
            .iter()
            .all(|run| *run == second)
    );
}

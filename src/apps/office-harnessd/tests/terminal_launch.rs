// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]
#![cfg(unix)]

//! The launcher end to end: a server with an empty environment finds a CLI
//! that only the login shell's `PATH` knows, the office additions win, and no
//! `AppImage` variable reaches the child.

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use agent_infrastructure::services::{Environment, LoginEnvironment};
use kernel::AgentId;
use office_harnessd::bridges::{
    AgentTerminalEvent, LaunchError, LaunchRequest, TerminalEvents, TerminalLauncher,
};
use tempfile::TempDir;
use terminal_application::common::errors::TerminalRuntimeError;
use terminal_application::common::interfaces::{OutputSubscriber, TerminalRuntime};
use terminal_application::common::models::{DeliveryStatus, LiveOutput};
use terminal_domain::errors::TerminalError;
use terminal_domain::events::TerminalEvent;
use terminal_domain::value_objects::{OutputKey, OutputLimits, TerminalSize};
use terminal_infrastructure::services::{PortableTerminalRuntime, RuntimeSettings};
use tokio::sync::{Mutex, MutexGuard, broadcast};

const MARKER: &str = "__OFFICE_HARNESS_ENVIRONMENT__";
const EVENT_WAIT: Duration = Duration::from_secs(15);

struct Silent;

impl OutputSubscriber for Silent {
    fn deliver(&self, _output: LiveOutput) -> DeliveryStatus {
        DeliveryStatus::Closed
    }
}

/// A script that was just written cannot be started while another thread
/// holds a forked copy of its still-open file ("text file busy"), and the
/// login shell then fails to start. The tests of this file write scripts and
/// start children, so they take turns for their whole run.
static ONE_TEST_AT_A_TIME: Mutex<()> = Mutex::const_new(());

fn write_executable(path: &Path, script: &str) {
    let mut file = fs::File::create(path).expect("create script");
    file.write_all(script.as_bytes()).expect("write script");
    file.sync_all().expect("sync script");
    drop(file);
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("make executable");
}

/// A login shell that answers with a `PATH` holding only `tool_folder`, and
/// with the server's `AppImage` variable that a real shell would inherit.
fn stand_in_login_shell(folder: &Path, tool_folder: &Path) -> std::path::PathBuf {
    let shell = folder.join("login-shell");
    let script = format!(
        "#!/bin/sh\nprintf '%s' '{MARKER}'\nprintf 'PATH={}\\0OFFICE_MARK=from-login\\0APPDIR=/appimage/mount\\0OLDPWD=/before\\0'\nprintf '%s' '{MARKER}'\n",
        tool_folder.display()
    );
    write_executable(&shell, &script);
    shell
}

/// A launcher for a server with an empty environment: the login shell
/// stand-in knows a folder of tools, and the server's own environment holds
/// only an `AppImage` variable.
struct Launch {
    _turn: MutexGuard<'static, ()>,
    folder: TempDir,
    tool_folder: PathBuf,
    runtime: Arc<PortableTerminalRuntime>,
    received: broadcast::Receiver<AgentTerminalEvent>,
    launcher: TerminalLauncher,
}

async fn launch_setup(tool_folder_name: &str) -> Launch {
    let turn = ONE_TEST_AT_A_TIME.lock().await;
    let folder = TempDir::new().expect("folder");
    let tool_folder = folder.path().join(tool_folder_name);
    fs::create_dir_all(&tool_folder).expect("tools folder");
    let events = TerminalEvents::new();
    let received = events.subscribe();
    let runtime = Arc::new(PortableTerminalRuntime::new(
        RuntimeSettings {
            output_folder: folder.path().join("output"),
            limits: OutputLimits::with_lines(1000).expect("limits"),
            quiet_period: Duration::from_secs(5),
        },
        Arc::new(events),
    ));
    let login = LoginEnvironment::with_shells(
        Some(stand_in_login_shell(folder.path(), &tool_folder)),
        None,
        Duration::from_secs(5),
    );
    let server_environment =
        Environment::from([("APPDIR".to_owned(), "/appimage/mount".to_owned())]);
    let launcher = TerminalLauncher::new(runtime.clone(), Arc::new(login), server_environment);
    Launch {
        _turn: turn,
        folder,
        tool_folder,
        runtime,
        received,
        launcher,
    }
}

fn request(
    launch: &Launch,
    program: &str,
    arguments: &[&str],
    additions: &[(&str, &str)],
) -> LaunchRequest {
    let agent_id = AgentId::generate();
    LaunchRequest {
        agent_id,
        program: program.to_owned(),
        arguments: arguments
            .iter()
            .map(|argument| (*argument).to_owned())
            .collect(),
        working_folder: launch.folder.path().to_path_buf(),
        additions: additions
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect(),
        size: TerminalSize::new(120, 24).expect("size"),
        output_key: OutputKey::parse(&agent_id.to_string()).expect("key"),
    }
}

/// Launches, waits for the exit, and returns what the child printed.
async fn run_to_exit(launch: &mut Launch, request: LaunchRequest) -> String {
    let agent = request.agent_id;
    let key = request.output_key.clone();
    let outcome = launch.launcher.launch(request).await.expect("launch");
    assert_eq!(outcome.environment_warning, None);
    loop {
        let item = tokio::time::timeout(EVENT_WAIT, launch.received.recv())
            .await
            .expect("an event in time")
            .expect("event channel open");
        if item.agent_id == agent && matches!(item.event, TerminalEvent::Exited { exit_code: 0 }) {
            break;
        }
    }
    let attachment = launch
        .runtime
        .attach(
            agent,
            Some(&key),
            TerminalSize::new(120, 24).expect("size"),
            Arc::new(Silent),
        )
        .expect("saved output");
    String::from_utf8_lossy(&attachment.snapshot).into_owned()
}

fn write_fake_cli(launch: &Launch) {
    write_executable(
        &launch.tool_folder.join("fake-cli"),
        "#!/bin/sh\necho \"mark=[$OFFICE_MARK] appdir=[$APPDIR] extra=[$OFFICE_EXTRA] term=[$TERM] color=[$COLORTERM] oldpwd=[$OLDPWD]\"\n",
    );
}

#[tokio::test]
async fn a_server_with_an_empty_environment_runs_a_cli_that_only_the_login_path_knows() {
    let mut launch = launch_setup("tools").await;
    write_fake_cli(&launch);
    let additions = [("OFFICE_MARK", "from-additions"), ("OFFICE_EXTRA", "yes")];

    let started = request(&launch, "fake-cli", &[], &additions);
    let text = run_to_exit(&mut launch, started).await;

    assert!(text.contains("mark=[from-additions]"), "{text}");
    assert!(text.contains("extra=[yes]"), "{text}");
    assert!(text.contains("appdir=[]"), "{text}");
}

#[tokio::test]
async fn the_agent_gets_a_terminal_type_that_the_additions_may_replace() {
    let mut launch = launch_setup("tools").await;
    write_fake_cli(&launch);

    let started = request(&launch, "fake-cli", &[], &[]);
    let default = run_to_exit(&mut launch, started).await;
    assert!(default.contains("term=[xterm-256color]"), "{default}");
    assert!(default.contains("color=[truecolor]"), "{default}");

    let started = request(&launch, "fake-cli", &[], &[("TERM", "vt100")]);
    let replaced = run_to_exit(&mut launch, started).await;
    assert!(replaced.contains("term=[vt100]"), "{replaced}");
}

#[tokio::test]
async fn the_shell_session_variables_do_not_reach_the_agent() {
    let mut launch = launch_setup("tools").await;
    write_fake_cli(&launch);

    let started = request(&launch, "fake-cli", &[], &[]);
    let text = run_to_exit(&mut launch, started).await;

    assert!(text.contains("oldpwd=[]"), "{text}");
}

#[tokio::test]
async fn a_batch_script_with_a_shell_operator_in_an_argument_is_refused_before_it_starts() {
    let launch = launch_setup("tools").await;
    write_executable(&launch.tool_folder.join("fake.cmd"), "#!/bin/sh\n");

    for argument in ["50%", "a&b", "a|b", "(x)"] {
        let error = launch
            .launcher
            .launch(request(&launch, "fake.cmd", &["--ok", argument], &[]))
            .await
            .expect_err("refused");
        assert!(
            matches!(
                error,
                LaunchError::Runtime(TerminalRuntimeError::Invalid(
                    TerminalError::UnsafeBatchArgument { index: 1, .. }
                ))
            ),
            "{argument}: {error}"
        );
    }
}

#[tokio::test]
async fn a_batch_script_in_a_folder_with_a_shell_operator_is_refused() {
    let launch = launch_setup("tools&more").await;
    write_executable(&launch.tool_folder.join("fake.cmd"), "#!/bin/sh\n");

    let error = launch
        .launcher
        .launch(request(&launch, "fake.cmd", &[], &[]))
        .await
        .expect_err("refused");

    assert!(matches!(
        error,
        LaunchError::Runtime(TerminalRuntimeError::Invalid(
            TerminalError::UnsafeBatchProgram { character: '&' }
        ))
    ));
}

#[tokio::test]
async fn a_program_that_is_on_no_path_is_reported_by_name() {
    let launch = launch_setup("tools").await;

    let error = launch
        .launcher
        .launch(request(&launch, "no-such-cli", &[], &[]))
        .await
        .expect_err("not found");

    assert!(matches!(error, LaunchError::ProgramNotFound(name) if name == "no-such-cli"));
}

// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

const INTERPRETER: &str = "C:\\Windows\\System32\\cmd.exe";

#[test]
fn a_plain_command_runs_as_it_is() {
    let arguments = vec!["--x".to_owned()];
    assert_eq!(
        launch_line("claude", &arguments, Some(INTERPRETER.to_owned())),
        ("claude".to_owned(), arguments.clone())
    );
    assert_eq!(
        launch_line("claude.cmd", &arguments, None),
        ("claude.cmd".to_owned(), arguments)
    );
}

#[test]
fn a_batch_script_on_windows_runs_through_the_interpreter_by_its_full_path() {
    let (program, arguments) = launch_line(
        "claude.cmd",
        &["--x".to_owned()],
        Some(INTERPRETER.to_owned()),
    );
    assert_eq!(program, INTERPRETER);
    assert_eq!(arguments, ["/d", "/c", "claude.cmd", "--x"]);
}

#[test]
fn the_interpreter_path_uses_the_system_root_of_the_environment() {
    let environment = vec![("SYSTEMROOT".to_owned(), "D:\\Win".to_owned())];
    assert_eq!(
        batch_interpreter(&environment),
        "D:\\Win\\System32\\cmd.exe"
    );
}

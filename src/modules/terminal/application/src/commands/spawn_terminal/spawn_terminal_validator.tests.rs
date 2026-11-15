// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use kernel::AgentId;
use terminal_domain::value_objects::{OutputKey, TerminalSize};

use super::*;

fn request(program: &str, arguments: &[&str]) -> SpawnTerminalRequest {
    SpawnTerminalRequest {
        agent_id: AgentId::generate(),
        program: program.to_owned(),
        arguments: arguments
            .iter()
            .map(|argument| (*argument).to_owned())
            .collect(),
        working_folder: std::env::temp_dir(),
        environment: Vec::new(),
        size: TerminalSize::new(80, 24).expect("valid size"),
        output_key: OutputKey::parse("session-1").expect("valid key"),
    }
}

#[test]
fn accepts_a_plain_command() {
    assert_eq!(
        validate_spawn_terminal(&request("claude", &["--x"])),
        Ok(())
    );
}

#[test]
fn refuses_an_empty_program() {
    assert_eq!(
        validate_spawn_terminal(&request(" ", &[])),
        Err(TerminalError::Empty { field: "program" })
    );
}

#[test]
fn refuses_a_relative_working_folder() {
    let mut relative = request("claude", &[]);
    relative.working_folder = PathBuf::from("here");
    assert_eq!(
        validate_spawn_terminal(&relative),
        Err(TerminalError::RelativePath {
            field: "working_folder"
        })
    );
}

#[test]
fn refuses_unsafe_arguments_for_a_batch_script_only() {
    assert_eq!(
        validate_spawn_terminal(&request("claude.cmd", &["--ok", "a%b"])),
        Err(TerminalError::UnsafeBatchArgument {
            index: 1,
            character: '%'
        })
    );
    assert_eq!(
        validate_spawn_terminal(&request("claude", &["a%b"])),
        Ok(())
    );
}

#[test]
fn refuses_a_batch_script_path_that_the_command_interpreter_would_act_on() {
    assert_eq!(
        validate_spawn_terminal(&request("C:\\tools\\a&b\\claude.cmd", &[])),
        Err(TerminalError::UnsafeBatchProgram { character: '&' })
    );
}

#[test]
fn refuses_shell_operators_in_batch_script_arguments() {
    for operator in ["a&b", "a|b", "a>b", "a<b", "a^b", "(a)"] {
        assert!(validate_spawn_terminal(&request("claude.bat", &[operator])).is_err());
    }
}

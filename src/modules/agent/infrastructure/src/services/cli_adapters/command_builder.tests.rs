// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn plan<'a>(fixed: &'a [String], flags: &'a [String], prompt: Option<&'a str>) -> SessionPlan<'a> {
    SessionPlan {
        fixed_arguments: fixed,
        flags,
        startup_prompt: prompt,
        proposed_cli_session_id: "id-1",
    }
}

fn words(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

#[test]
fn fixed_arguments_come_before_flags_then_the_adapter_arguments() {
    let fixed = words(&["--model", "opus"]);
    let flags = words(&["--verbose"]);

    let command = new_session_command(
        &plan(&fixed, &flags, Some("go now")),
        &["--session-id", "id-1"],
        Some("id-1".to_owned()),
    );

    assert_eq!(
        command.arguments,
        ["--model", "opus", "--verbose", "--session-id", "id-1"]
    );
    assert_eq!(command.cli_session_id.as_deref(), Some("id-1"));
}

#[test]
fn every_prompt_is_typed_in_and_never_an_argument() {
    for prompt in ["go", "logout", "go now", "--dangerously-skip"] {
        let command = new_session_command(&plan(&[], &[], Some(prompt)), &[], None);

        assert_eq!(command.arguments, Vec::<String>::new(), "{prompt}");
        assert_eq!(command.typed_input.as_deref(), Some(prompt));
    }
}

#[test]
fn no_prompt_means_nothing_is_typed() {
    let command = new_session_command(&plan(&[], &[], None), &[], None);

    assert_eq!(command.typed_input, None);
}

#[test]
fn a_continue_never_carries_the_prompt() {
    let command = continue_command(&plan(&[], &[], Some("go")), &["--resume", "id-1"]);

    assert_eq!(command.arguments, ["--resume", "id-1"]);
    assert_eq!(command.typed_input, None);
}

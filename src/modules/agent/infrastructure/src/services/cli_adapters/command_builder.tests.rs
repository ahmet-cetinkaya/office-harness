// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn plan<'a>(
    fixed: &'a [String],
    flags: &'a [String],
    prompt: Option<&'a str>,
    by_argument: bool,
) -> SessionPlan<'a> {
    SessionPlan {
        fixed_arguments: fixed,
        flags,
        startup_prompt: prompt,
        prompt_by_argument: by_argument,
        proposed_cli_session_id: "id-1",
    }
}

fn words(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

#[test]
fn fixed_arguments_come_before_flags_then_the_adapter_then_the_prompt() {
    let fixed = words(&["--model", "opus"]);
    let flags = words(&["--verbose"]);

    let command = new_session_command(
        &plan(&fixed, &flags, Some("go now"), true),
        &["--session-id", "id-1"],
        PromptStyle::Positional,
        Some("id-1".to_owned()),
    );

    assert_eq!(
        command.arguments,
        [
            "--model",
            "opus",
            "--verbose",
            "--session-id",
            "id-1",
            "go now"
        ]
    );
    assert_eq!(command.cli_session_id.as_deref(), Some("id-1"));
    assert_eq!(command.typed_input, None);
}

#[test]
fn a_flag_style_prompt_follows_its_flag() {
    let command = new_session_command(
        &plan(&[], &[], Some("go"), true),
        &[],
        PromptStyle::Flag("--prompt"),
        None,
    );

    assert_eq!(command.arguments, ["--prompt", "go"]);
}

#[test]
fn a_prompt_is_typed_when_the_program_is_a_batch_script_or_the_cli_has_no_argument() {
    let batch = new_session_command(
        &plan(&[], &[], Some("go"), false),
        &[],
        PromptStyle::Positional,
        None,
    );
    let typed_only = new_session_command(
        &plan(&[], &[], Some("go"), true),
        &[],
        PromptStyle::Typed,
        None,
    );

    for command in [batch, typed_only] {
        assert_eq!(command.arguments, Vec::<String>::new());
        assert_eq!(command.typed_input.as_deref(), Some("go"));
    }
}

#[test]
fn a_one_word_positional_prompt_is_typed_so_it_cannot_read_as_a_subcommand() {
    let command = new_session_command(
        &plan(&[], &[], Some("logout"), true),
        &[],
        PromptStyle::Positional,
        None,
    );

    assert_eq!(command.arguments, Vec::<String>::new());
    assert_eq!(command.typed_input.as_deref(), Some("logout"));
}

#[test]
fn a_prompt_that_starts_with_a_dash_is_typed_so_it_cannot_read_as_a_flag() {
    let command = new_session_command(
        &plan(&[], &[], Some("--dangerously-skip"), true),
        &[],
        PromptStyle::Positional,
        None,
    );

    assert_eq!(command.arguments, Vec::<String>::new());
    assert_eq!(command.typed_input.as_deref(), Some("--dangerously-skip"));
}

#[test]
fn a_continue_never_carries_the_prompt() {
    let command = continue_command(&plan(&[], &[], Some("go"), true), &["--resume", "id-1"]);

    assert_eq!(command.arguments, ["--resume", "id-1"]);
    assert_eq!(command.typed_input, None);
}

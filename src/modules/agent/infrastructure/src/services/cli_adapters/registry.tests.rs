// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::common::models::SessionPlan;

use super::*;

fn registry() -> CliAdapterRegistry {
    CliAdapterRegistry::new(AttributionTiming::default())
}

fn new_session_arguments(program: &str, prompt: Option<&str>) -> Vec<String> {
    let flags = ["--flag".to_owned()];
    let plan = SessionPlan {
        fixed_arguments: &[],
        flags: &flags,
        startup_prompt: prompt,
        prompt_by_argument: true,
        proposed_cli_session_id: "id-1",
    };
    registry().for_program(program).new_session(&plan).arguments
}

fn continue_arguments(program: &str) -> Option<Vec<String>> {
    let plan = SessionPlan {
        fixed_arguments: &[],
        flags: &[],
        startup_prompt: Some("ignored"),
        prompt_by_argument: true,
        proposed_cli_session_id: "id-2",
    };
    registry()
        .for_program(program)
        .continue_session(&plan, "cli-9")
        .map(|command| command.arguments)
}

#[test]
fn each_adapter_builds_its_documented_new_session_command_line() {
    assert_eq!(
        new_session_arguments("claude", Some("say hi")),
        ["--flag", "--session-id", "id-1", "say hi"]
    );
    assert_eq!(
        new_session_arguments("codex", Some("say hi")),
        ["--flag", "say hi"]
    );
    assert_eq!(
        new_session_arguments("gemini", Some("say hi")),
        ["--flag", "--session-id", "id-1", "-i", "say hi"]
    );
    assert_eq!(
        new_session_arguments("opencode", Some("say hi")),
        ["--flag", "--session", "id-1", "--prompt", "say hi"]
    );
    assert_eq!(
        new_session_arguments("some-tool", Some("say hi")),
        ["--flag"]
    );
}

#[test]
fn each_adapter_builds_its_documented_continue_command_line() {
    assert_eq!(
        continue_arguments("claude"),
        Some(vec!["--resume".into(), "cli-9".into()])
    );
    assert_eq!(
        continue_arguments("codex"),
        Some(vec!["resume".into(), "cli-9".into()])
    );
    assert_eq!(
        continue_arguments("gemini"),
        Some(vec!["--resume".into(), "cli-9".into()])
    );
    assert_eq!(
        continue_arguments("opencode"),
        Some(vec!["--session".into(), "cli-9".into()])
    );
}

#[test]
fn the_generic_adapter_cannot_continue_and_types_the_prompt() {
    let generic = registry().for_program("some-tool");
    let plan = SessionPlan {
        fixed_arguments: &[],
        flags: &[],
        startup_prompt: Some("say hi"),
        prompt_by_argument: true,
        proposed_cli_session_id: "id-1",
    };

    assert!(!generic.supports_continue());
    assert_eq!(continue_arguments("some-tool"), None);
    assert_eq!(
        generic.new_session(&plan).typed_input.as_deref(),
        Some("say hi")
    );
}

#[test]
fn the_adapter_follows_the_program_name_without_folder_extension_or_case() {
    for program in ["/usr/bin/claude", "Claude.CMD", "claude.exe"] {
        assert!(
            new_session_arguments(program, None).contains(&"--session-id".to_owned()),
            "{program}"
        );
    }
    assert_eq!(new_session_arguments("claude-wrapper", None), ["--flag"]);
}

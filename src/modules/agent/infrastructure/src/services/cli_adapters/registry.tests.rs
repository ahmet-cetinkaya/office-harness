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
        proposed_cli_session_id: "id-1",
    };
    registry().for_program(program).new_session(&plan).arguments
}

fn continue_arguments(program: &str) -> Option<Vec<String>> {
    let plan = SessionPlan {
        fixed_arguments: &[],
        flags: &[],
        startup_prompt: Some("ignored"),
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
        ["--flag", "--session-id", "id-1"]
    );
    assert_eq!(new_session_arguments("codex", Some("say hi")), ["--flag"]);
    assert_eq!(
        new_session_arguments("gemini", Some("say hi")),
        ["--flag", "--session-id", "id-1"]
    );
    let opencode = new_session_arguments("opencode", Some("say hi"));
    assert_eq!(opencode[..2], ["--flag", "--session"]);
    assert!(opencode[2].starts_with("ses_"), "{opencode:?}");
    assert_eq!(
        new_session_arguments("some-tool", Some("say hi")),
        ["--flag"]
    );
}

#[test]
fn opencode_starts_and_records_a_session_id_in_its_own_ses_shape() {
    // A plain UUID stops it with "Expected a string starting with \"ses\"", and
    // an id whose counter is not near the present time makes the free tier
    // answer "Free usage limit reached" (2.2.5).
    let plan = SessionPlan {
        fixed_arguments: &[],
        flags: &[],
        startup_prompt: None,
        proposed_cli_session_id: "c54938eb-c04e-4ff7-bd9c-06f1c9375f91",
    };

    let command = registry().for_program("opencode").new_session(&plan);

    let recorded = command
        .cli_session_id
        .as_deref()
        .expect("an id is recorded");
    assert_eq!(command.arguments, ["--session", recorded]);
    assert_eq!(recorded.len(), 4 + 26);
    assert!(recorded.starts_with("ses_"));
    assert!(recorded.ends_with("938ebc04e4ff7b"), "{recorded}");
}

#[test]
fn every_adapter_types_the_prompt_in() {
    for program in ["claude", "codex", "gemini", "opencode", "some-tool"] {
        let plan = SessionPlan {
            fixed_arguments: &[],
            flags: &[],
            startup_prompt: Some("say hi"),
            proposed_cli_session_id: "id-1",
        };

        let command = registry().for_program(program).new_session(&plan);

        assert_eq!(command.typed_input.as_deref(), Some("say hi"), "{program}");
    }
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

#[test]
fn claude_and_codex_have_a_status_command_and_gemini_only_screen_words() {
    let claude = registry().for_program("claude").sign_in();
    let status = claude.status.expect("claude status");
    assert_eq!(status.arguments, ["auth", "status", "--text"]);
    assert_eq!(status.not_signed_in_words, ["not logged in"]);
    assert_eq!(claude.cli, "claude");

    let codex = registry().for_program("/usr/bin/codex").sign_in();
    assert_eq!(
        codex.status.expect("codex status").arguments,
        ["login", "status"]
    );
    assert_eq!(
        codex.screen_words,
        ["sign in with chatgpt", "provide your own api key"]
    );

    let gemini = registry().for_program("gemini").sign_in();
    assert_eq!(gemini.status, None);
    assert!(
        gemini
            .screen_words
            .contains(&"how would you like to authenticate")
    );

    for program in ["opencode", "bash"] {
        assert_eq!(
            registry().for_program(program).sign_in(),
            agent_application::common::models::SignIn::default()
        );
    }
}

#[test]
fn every_word_is_lower_case_so_a_case_free_match_finds_it() {
    for program in ["claude", "codex", "gemini"] {
        let sign_in = registry().for_program(program).sign_in();
        let status_words = sign_in
            .status
            .map_or(&[][..], |status| status.not_signed_in_words);
        for word in sign_in.screen_words.iter().chain(status_words) {
            assert_eq!(*word, word.to_lowercase(), "{program}: {word}");
        }
    }
}

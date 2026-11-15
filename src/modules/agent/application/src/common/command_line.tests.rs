// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn split(text: &str) -> Result<(String, Vec<String>), ApplicationError> {
    split_command(&AgentCommand::parse(text).expect("command"))
}

#[test]
fn the_first_word_is_the_program_and_the_rest_are_arguments() {
    let (program, arguments) = split("  claude --model  opus ").expect("split");

    assert_eq!(program, "claude");
    assert_eq!(arguments, ["--model", "opus"]);
}

#[test]
fn a_bare_program_has_no_arguments() {
    assert_eq!(
        split("codex").expect("split"),
        ("codex".to_owned(), Vec::new())
    );
}

#[test]
fn double_quotes_hold_white_space() {
    let (program, arguments) =
        split(r#"claude --append-system-prompt "be very brief" -p"#).expect("split");

    assert_eq!(program, "claude");
    assert_eq!(arguments, ["--append-system-prompt", "be very brief", "-p"]);
}

#[test]
fn the_program_may_be_quoted_with_spaces_in_its_path() {
    let (program, arguments) = split(r#""/opt/my tools/cli" run"#).expect("split");

    assert_eq!(program, "/opt/my tools/cli");
    assert_eq!(arguments, ["run"]);
}

#[test]
fn a_backslash_before_a_quote_escapes_it_and_pairs_collapse() {
    let (_, arguments) = split(r#"cli "say \"hi\"" "end\\\\" x"#).expect("split");

    assert_eq!(arguments, [r#"say "hi""#, r"end\\", "x"]);
}

#[test]
fn a_quoted_path_that_ends_in_a_backslash_writes_it_twice() {
    let (program, arguments) = split(r#""C:\dir\\" next"#).expect("split");

    assert_eq!(program, r"C:\dir\");
    assert_eq!(arguments, ["next"]);
}

#[test]
fn a_quoted_path_that_ends_in_one_backslash_leaves_the_quote_open() {
    let error = split(r#""C:\dir\""#).expect_err("escaped quote");

    assert!(matches!(&error, ApplicationError::Invalid(text) if text.contains("never closed")));
}

#[test]
fn a_unc_path_is_kept_with_or_without_quotes() {
    let (program, arguments) =
        split(r#"\\server\share\tool.exe "\\file server\share\x""#).expect("split");

    assert_eq!(program, r"\\server\share\tool.exe");
    assert_eq!(arguments, [r"\\file server\share\x"]);
}

#[test]
fn a_backslash_before_a_space_is_literal() {
    let (_, arguments) = split(r"cli a\ b").expect("split");

    assert_eq!(arguments, [r"a\", "b"]);
}

#[test]
fn an_empty_pair_of_quotes_is_an_empty_argument() {
    let (_, arguments) = split(r#"cli "" x"#).expect("split");

    assert_eq!(arguments, ["", "x"]);
}

#[test]
fn nothing_is_expanded() {
    let (_, arguments) = split("cli $HOME *.rs ~").expect("split");

    assert_eq!(arguments, ["$HOME", "*.rs", "~"]);
}

#[test]
fn a_quote_that_is_never_closed_is_invalid() {
    let error = split(r#"cli "unfinished"#).expect_err("open quote");

    assert!(matches!(&error, ApplicationError::Invalid(text) if text.contains("command")));
}

#[test]
fn an_empty_program_is_invalid() {
    let error = split(r#""" arg"#).expect_err("empty program");

    assert!(matches!(&error, ApplicationError::Invalid(text) if text.contains("program")));
}

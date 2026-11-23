// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

const COMMAND: StatusCommand = StatusCommand {
    arguments: &["auth", "status"],
    not_signed_in_words: &["not logged in"],
    key_variables: &["SIGNIN_API_KEY"],
};

const CLAUDE: SignIn = SignIn {
    cli: "claude",
    status: Some(COMMAND),
    screen_words: &[],
};

fn ended(exit_code: Option<i32>, output: &str) -> StatusRunResult {
    StatusRunResult::Ended {
        exit_code,
        output: output.to_owned(),
    }
}

fn answer_of(result: &StatusRunResult) -> SignInAnswer {
    answer(result, &COMMAND, SIGN_IN_TIMEOUT)
}

#[test]
fn exit_zero_is_signed_in_even_with_the_words() {
    assert_eq!(answer_of(&ended(Some(0), "")), SignInAnswer::SignedIn);
    assert_eq!(
        answer_of(&ended(Some(0), "Not logged in")),
        SignInAnswer::SignedIn
    );
}

#[test]
fn a_non_zero_exit_with_the_words_is_not_signed_in_with_that_line() {
    let output = "{\n\nNot logged in. Run claude auth login to authenticate.\nmore";
    assert_eq!(
        answer_of(&ended(Some(1), output)),
        SignInAnswer::NotSignedIn {
            line: "Not logged in. Run claude auth login to authenticate.".to_owned()
        }
    );
}

#[test]
fn a_non_zero_exit_without_the_words_is_unclear() {
    let usage = "error: unrecognized subcommand 'status'\nUsage: claude [OPTIONS]";
    for result in [
        ended(Some(2), usage),
        ended(Some(1), ""),
        ended(None, "killed"),
    ] {
        assert_eq!(
            answer_of(&result),
            SignInAnswer::NoAnswer(NoAnswerReason::Unclear)
        );
    }
}

#[test]
fn a_signal_end_with_the_words_is_not_signed_in() {
    assert!(matches!(
        answer_of(&ended(None, "not logged in")),
        SignInAnswer::NotSignedIn { .. }
    ));
}

#[test]
fn a_timeout_and_a_failed_start_give_no_answer() {
    assert_eq!(
        answer_of(&StatusRunResult::TimedOut),
        SignInAnswer::NoAnswer(NoAnswerReason::TimedOut(SIGN_IN_TIMEOUT))
    );
    assert_eq!(
        answer_of(&StatusRunResult::CouldNotRun(
            "not found\u{1b}[2J".to_owned()
        )),
        SignInAnswer::NoAnswer(NoAnswerReason::CouldNotRun("not found␛[2J".to_owned()))
    );
}

#[test]
fn a_quoted_line_is_visible_and_capped() {
    let line = quoted_line("  \u{1b}[31mNot logged in\u{202e}\u{7f}\u{9b}  ");
    assert_eq!(line, "␛[31mNot logged in\\u202E␡\\u009B");

    let long = "ş".repeat(500);
    let cut = quoted_line(&long);
    assert_eq!(cut.chars().count(), QUOTED_LINE_MAX_CHARS);
    assert!(cut.ends_with('…'));

    let exact = "a".repeat(QUOTED_LINE_MAX_CHARS);
    assert_eq!(quoted_line(&exact), exact);
}

#[test]
fn the_quoted_line_is_the_one_with_the_word_and_nothing_else() {
    let output = "token sk-secret-value\nNOT LOGGED IN\nanother line";
    let SignInAnswer::NotSignedIn { line } = answer_of(&ended(Some(1), output)) else {
        panic!("expected not signed in");
    };
    assert_eq!(line, "NOT LOGGED IN");
    let reason = not_signed_in_reason(&CLAUDE, NotSignedInSource::StatusCommand(&COMMAND), &line);
    assert!(!reason.contains("sk-secret-value"));
    assert!(!reason.contains("another line"));
}

#[test]
fn the_reason_names_the_cli_how_it_was_found_and_what_to_do() {
    assert_eq!(
        not_signed_in_reason(
            &CLAUDE,
            NotSignedInSource::StatusCommand(&COMMAND),
            "Not logged in"
        ),
        "claude is not signed in: `claude auth status` said so. Sign in to claude in your own \
         terminal, then start the agent again. It showed: Not logged in"
    );
    assert_eq!(
        not_signed_in_reason(&CLAUDE, NotSignedInSource::SignInScreen, "Select"),
        "claude is not signed in: its first screen asks to sign in. Sign in to claude in your \
         own terminal, then start the agent again. It showed: Select"
    );
}

#[test]
fn the_no_answer_note_says_why_without_output() {
    assert_eq!(
        no_answer_note(&NoAnswerReason::TimedOut(SIGN_IN_TIMEOUT)),
        "the sign-in check gave no answer: it timed out after 5 s"
    );
    assert_eq!(
        no_answer_note(&NoAnswerReason::CouldNotRun("not found".to_owned())),
        "the sign-in check gave no answer: it could not run: not found"
    );
    assert_eq!(
        no_answer_note(&NoAnswerReason::Unclear),
        "the sign-in check gave no answer: its answer was unclear"
    );
    assert_eq!(
        no_answer_note(&NoAnswerReason::AgentChanged),
        "the sign-in check gave no answer: the agent changed during the check"
    );
    assert_eq!(
        answer_of(&StatusRunResult::Skipped),
        SignInAnswer::NotChecked
    );
}

#[test]
fn line_with_word_ignores_case() {
    assert_eq!(
        line_with_word(
            "a\nHow Would You Like To Authenticate?\n",
            &["how would you like"]
        ),
        Some("How Would You Like To Authenticate?")
    );
    assert_eq!(line_with_word("nothing here", &["sign in"]), None);
}

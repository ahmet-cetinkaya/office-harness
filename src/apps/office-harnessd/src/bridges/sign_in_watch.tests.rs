// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

const WORDS: &[&str] = &["how would you like to authenticate"];

fn screen(rows: &[&str]) -> ScreenView {
    ScreenView {
        rows: rows.iter().map(|row| (*row).to_owned()).collect(),
        cursor_row: rows.len().saturating_sub(1),
        cursor_column: 0,
    }
}

#[test]
fn a_sign_in_row_at_the_bottom_is_found_and_quoted() {
    let shown = screen(&[
        "Gemini CLI",
        "",
        "\u{1b}How Would You Like To Authenticate for this project?",
        "",
    ]);
    assert_eq!(
        sign_in_row(&shown, WORDS),
        Some("␛How Would You Like To Authenticate for this project?".to_owned())
    );
}

#[test]
fn a_sign_in_row_above_the_bottom_rows_is_not_found() {
    let mut rows = vec!["how would you like to authenticate"];
    rows.extend(std::iter::repeat_n("busy", SIGN_IN_ROWS));
    assert_eq!(sign_in_row(&screen(&rows), WORDS), None);
}

#[test]
fn a_screen_without_the_words_is_not_a_sign_in_screen() {
    assert_eq!(sign_in_row(&screen(&["> ready", "tips"]), WORDS), None);
}

#[test]
fn an_end_belongs_to_its_run_only_and_is_taken_once() {
    let ends = SignInEnds::default();
    let agent_id = AgentId::generate();
    let run = RunId(1);
    ends.mark(agent_id, run, "reason".to_owned());

    assert_eq!(ends.take(agent_id, RunId(2)), None);
    assert_eq!(ends.take(agent_id, run), Some("reason".to_owned()));
    assert_eq!(ends.take(agent_id, run), None);
}

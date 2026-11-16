// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn the_english_question_is_answered_with_its_first_letter() {
    assert_eq!(confirmation_answer("Terminate batch job (Y/N)?"), Some('Y'));
}

#[test]
fn the_turkish_question_and_trailing_spaces_are_accepted() {
    assert_eq!(
        confirmation_answer("Toplu is sonlandirilsin mi (E/H)?   "),
        Some('E')
    );
}

#[test]
fn a_line_that_is_not_such_a_question_gets_no_answer() {
    for line in [
        "",
        "Terminate batch job (Y/N)",
        "(Y/N)? later",
        "(YES/NO)?",
        "(1/2)?",
        "(Y/)?",
        "(/N)?",
        "Y/N)?",
        "ready?",
    ] {
        assert_eq!(confirmation_answer(line), None, "{line:?}");
    }
}

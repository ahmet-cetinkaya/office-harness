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

#[test]
fn a_question_that_appears_after_the_interrupt_is_answered() {
    assert_eq!(
        answer_after_interrupt("C:\\work>", "Terminate batch job (Y/N)?"),
        Some('Y')
    );
}

#[test]
fn a_question_that_was_on_the_screen_before_the_interrupt_gets_no_answer() {
    let own_question = "Overwrite the file (y/n)?";
    assert_eq!(answer_after_interrupt(own_question, own_question), None);
}

#[test]
fn a_redraw_of_the_programs_own_question_gets_no_answer() {
    assert_eq!(
        answer_after_interrupt("Overwrite the file (y/n)?  ", "  Overwrite the file (y/n)?"),
        None
    );
}

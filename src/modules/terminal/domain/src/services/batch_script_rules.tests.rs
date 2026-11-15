// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn recognizes_batch_scripts_in_any_case() {
    assert!(is_batch_script("C:\\tools\\claude.CMD"));
    assert!(is_batch_script("run.bat"));
    assert!(!is_batch_script("claude.exe"));
}

#[test]
fn reports_the_first_unsafe_argument() {
    let arguments = ["--ok".to_owned(), "50%".to_owned(), "a\"b".to_owned()];
    assert_eq!(first_unsafe_batch_argument(&arguments), Some((1, '%')));
    assert_eq!(first_unsafe_batch_argument(&arguments[..1]), None);
}

#[test]
fn refuses_every_character_that_the_command_interpreter_acts_on() {
    for character in ['%', '!', '"', '\r', '\n', '&', '|', '<', '>', '^', '(', ')'] {
        let text = format!("a{character}b");
        assert_eq!(first_unsafe_batch_character(&text), Some(character));
    }
    assert_eq!(first_unsafe_batch_character("--flag=value_1.2/x"), None);
}

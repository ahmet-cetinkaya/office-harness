// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn control_and_bidirectional_characters_become_visible_symbols() {
    assert_eq!(visible_text("\u{1b}[31m"), "␛[31m");
    assert_eq!(visible_text("one\ntwo\r"), "one␊two␍");
    assert_eq!(visible_text("\u{7f}"), "␡");
    assert_eq!(visible_text("\u{9b}2J"), "\\u009B2J");
    assert_eq!(visible_text("\u{202e}evil"), "\\u202Eevil");
    assert_eq!(visible_text("\u{2066}x\u{2069}"), "\\u2066x\\u2069");
}

#[test]
fn plain_text_stays_as_it_is() {
    assert_eq!(visible_text("plain text ✓ çğü"), "plain text ✓ çğü");
}

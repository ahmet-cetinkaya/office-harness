// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn random_bytes_become_64_lowercase_hex_characters() {
    let token = OfficeToken::from_random_bytes([0xAB; RANDOM_BYTE_COUNT]);

    assert_eq!(token.expose_secret(), "ab".repeat(RANDOM_BYTE_COUNT));
}

#[test]
fn the_text_of_a_token_parses_back_with_or_without_a_line_break() {
    let token = OfficeToken::from_random_bytes([7; RANDOM_BYTE_COUNT]);
    let text = token.expose_secret().to_owned();

    assert_eq!(OfficeToken::parse(&text), Ok(token.clone()));
    assert_eq!(OfficeToken::parse(&format!("{text}\n")), Ok(token));
}

#[test]
fn text_that_is_not_a_token_is_malformed() {
    let uppercase = "AB".repeat(RANDOM_BYTE_COUNT);
    let short = "ab".repeat(RANDOM_BYTE_COUNT - 1);
    let long = "ab".repeat(RANDOM_BYTE_COUNT + 1);
    let not_hex = "zz".repeat(RANDOM_BYTE_COUNT);

    for text in [
        "",
        " ",
        uppercase.as_str(),
        short.as_str(),
        long.as_str(),
        not_hex.as_str(),
    ] {
        assert_eq!(
            OfficeToken::parse(text),
            Err(TokenError::Malformed),
            "{text}"
        );
    }
}

#[test]
fn debug_output_and_the_error_never_show_the_value() {
    let token = OfficeToken::from_random_bytes([0xCD; RANDOM_BYTE_COUNT]);
    let secret = token.expose_secret().to_owned();
    let error = OfficeToken::parse(&format!("{secret}x")).expect_err("malformed");

    assert!(!format!("{token:?}").contains(&secret));
    assert!(!error.to_string().contains(&secret));
}

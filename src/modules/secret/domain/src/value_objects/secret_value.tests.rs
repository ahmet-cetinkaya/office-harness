// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn a_value_of_8_to_8192_bytes_without_nul_is_accepted() {
    for length in [MIN_VALUE_BYTES, 100, MAX_VALUE_BYTES] {
        let value = SecretValue::parse("k".repeat(length)).expect("value");
        assert_eq!(value.reveal().len(), length);
    }
}

#[test]
fn a_short_long_or_nul_value_is_refused_without_echoing_it() {
    let short = SecretValue::parse("sh0rt".to_owned()).expect_err("short");
    assert_eq!(short, SecretError::ValueTooShort { min: 8 });
    assert!(!short.to_string().contains("sh0rt"));
    assert_eq!(
        SecretValue::parse("k".repeat(MAX_VALUE_BYTES + 1)).expect_err("long"),
        SecretError::ValueTooLong {
            max: MAX_VALUE_BYTES
        }
    );
    let nul = SecretValue::parse("marker-\0-value".to_owned()).expect_err("nul");
    assert_eq!(nul, SecretError::ValueHasNul);
    assert!(!nul.to_string().contains("marker"));
}

#[test]
fn a_value_prints_as_a_placeholder() {
    let value = SecretValue::parse("unique-marker-value".to_owned()).expect("value");
    assert_eq!(format!("{value:?}"), "[secret]");
    assert_eq!(format!("{:#?}", Some(&value)), "Some(\n    [secret],\n)");
}

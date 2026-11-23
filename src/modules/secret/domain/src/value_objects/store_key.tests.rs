// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn a_key_needs_exactly_32_bytes_and_prints_as_a_placeholder() {
    assert!(StoreKey::from_slice(&[7; 31]).is_none());
    assert!(StoreKey::from_slice(&[7; 33]).is_none());
    let key = StoreKey::from_slice(&[7; 32]).expect("key");
    assert_eq!(format!("{key:?}"), "[secret]");
}

#[test]
fn two_keys_match_only_when_every_byte_is_equal() {
    let left = StoreKey::from_bytes([1; 32]);
    let same = StoreKey::from_bytes([1; 32]);
    let mut other_bytes = [1; 32];
    other_bytes[31] = 2;
    assert!(left.matches(&same));
    assert!(!left.matches(&StoreKey::from_bytes(other_bytes)));
}

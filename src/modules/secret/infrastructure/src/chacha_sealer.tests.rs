// SPDX-License-Identifier: AGPL-3.0-or-later

use secret_domain::{SecretId, SecretName, SecretVersion};

use super::*;

const MARKER: &str = "unique-marker-value-42";

fn binding(name: &str, version: u32) -> RecordBinding {
    RecordBinding {
        id: SecretId::from_stored("id-1".to_owned()),
        name: SecretName::parse(name).expect("name"),
        version: SecretVersion::from_stored(version),
    }
}

fn value() -> SecretValue {
    SecretValue::parse(MARKER.to_owned()).expect("value")
}

fn key(byte: u8) -> StoreKey {
    StoreKey::from_bytes([byte; 32])
}

#[test]
fn a_sealed_value_opens_to_the_same_value() {
    let sealer = ChaChaRecordSealer;
    let record = sealer
        .seal(&key(1), &binding("A_KEY", 1), &value())
        .expect("seal");

    let opened = sealer
        .open(&key(1), &binding("A_KEY", 1), &record)
        .expect("open");

    assert_eq!(opened.reveal(), MARKER);
}

#[test]
fn the_record_shows_nothing_readable_of_the_value() {
    let record = ChaChaRecordSealer
        .seal(&key(1), &binding("A_KEY", 1), &value())
        .expect("seal");

    let bytes = record.as_bytes();
    assert!(
        !bytes
            .windows(MARKER.len())
            .any(|window| window == MARKER.as_bytes())
    );
    assert!(
        !bytes
            .windows(8)
            .any(|window| window == b"A_KEY\0\0\0".as_slice())
    );
}

#[test]
fn two_seals_of_one_value_differ() {
    let sealer = ChaChaRecordSealer;
    let first = sealer
        .seal(&key(1), &binding("A_KEY", 1), &value())
        .expect("seal");
    let second = sealer
        .seal(&key(1), &binding("A_KEY", 1), &value())
        .expect("seal");

    assert_ne!(first, second);
}

#[test]
fn a_record_moved_to_another_name_or_put_back_after_a_rotation_does_not_open() {
    let sealer = ChaChaRecordSealer;
    let record = sealer
        .seal(&key(1), &binding("A_KEY", 1), &value())
        .expect("seal");

    assert_eq!(
        sealer
            .open(&key(1), &binding("OTHER_KEY", 1), &record)
            .expect_err("name"),
        SealError::CannotOpen
    );
    assert_eq!(
        sealer
            .open(&key(1), &binding("A_KEY", 2), &record)
            .expect_err("version"),
        SealError::CannotOpen
    );
}

#[test]
fn another_key_a_flipped_byte_and_a_short_record_do_not_open() {
    let sealer = ChaChaRecordSealer;
    let record = sealer
        .seal(&key(1), &binding("A_KEY", 1), &value())
        .expect("seal");
    let mut flipped = record.as_bytes().to_vec();
    let last = flipped.len() - 1;
    flipped[last] ^= 1;

    for bad in [
        sealer.open(&key(2), &binding("A_KEY", 1), &record),
        sealer.open(
            &key(1),
            &binding("A_KEY", 1),
            &SealedRecord::from_bytes(flipped),
        ),
        sealer.open(
            &key(1),
            &binding("A_KEY", 1),
            &SealedRecord::from_bytes(vec![1, 2, 3]),
        ),
    ] {
        assert_eq!(bad.expect_err("refused"), SealError::CannotOpen);
    }
}

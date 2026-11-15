// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn keeps_unix_milliseconds() {
    assert_eq!(
        Timestamp::from_unix_millis(1_700_000_000_000).as_unix_millis(),
        1_700_000_000_000
    );
}

#[test]
fn orders_by_time() {
    assert!(Timestamp::from_unix_millis(1) < Timestamp::from_unix_millis(2));
}

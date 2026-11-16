// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn caps_the_bytes_at_sixteen_mebibytes() {
    let limits = OutputLimits::new(10_000, usize::MAX).expect("valid limits");
    assert_eq!(limits.max_bytes(), MAX_OUTPUT_BYTES);
}

#[test]
fn refuses_a_zero_limit() {
    assert!(OutputLimits::new(0, 10).is_err());
    assert!(OutputLimits::new(10, 0).is_err());
}

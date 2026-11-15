// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn accepts_a_positive_size() {
    let size = TerminalSize::new(80, 24).expect("valid size");
    assert_eq!((size.columns().get(), size.rows().get()), (80, 24));
}

#[test]
fn names_the_side_that_is_zero() {
    assert_eq!(
        TerminalSize::new(0, 24),
        Err(TerminalError::NotPositive { field: "cols" })
    );
    assert_eq!(
        TerminalSize::new(80, 0),
        Err(TerminalError::NotPositive { field: "rows" })
    );
}

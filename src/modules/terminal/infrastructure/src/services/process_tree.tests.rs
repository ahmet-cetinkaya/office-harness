// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt::Write;

use super::unix::{MAX_DESCENDANTS, descendants_in};

#[test]
fn walks_children_and_grandchildren_only() {
    let table = "  1     0\n 10     1\n 11    10\n 12    11\n 20     1\n 13    10\n";
    let mut found = descendants_in(10, table);
    found.sort_unstable();
    assert_eq!(found, [11, 12, 13]);
}

#[test]
fn ignores_lines_that_are_not_pairs_of_numbers() {
    assert_eq!(descendants_in(10, "garbage\n 5\n a b\n"), Vec::new());
}

#[test]
fn stops_walking_at_the_cap() {
    let mut wide = String::new();
    for offset in 0..MAX_DESCENDANTS + 50 {
        writeln!(wide, "{} 1", offset + 10).expect("a string takes text");
    }
    assert_eq!(descendants_in(1, &wide).len(), MAX_DESCENDANTS);
}

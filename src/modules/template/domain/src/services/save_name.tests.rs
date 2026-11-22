// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn name(text: &str) -> TemplateName {
    TemplateName::parse(text).expect("name")
}

#[test]
fn a_free_name_is_kept() {
    assert_eq!(save_name(&name("Reviewer"), |_| false), name("Reviewer"));
}

#[test]
fn a_taken_name_gets_the_lowest_free_number_ignoring_case() {
    let taken = ["reviewer", "reviewer (2)"];

    let saved = save_name(&name("Reviewer"), |key| taken.contains(&key));

    assert_eq!(saved, name("Reviewer (3)"));
}

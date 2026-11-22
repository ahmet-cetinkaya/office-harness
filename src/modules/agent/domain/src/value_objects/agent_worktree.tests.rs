// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn names_git_refuses_become_valid_slugs() {
    assert_eq!(branch_slug("a..b"), "a-b");
    assert_eq!(branch_slug("x@{1}"), "x-1");
    assert_eq!(branch_slug("a/.b"), "a-b");
    assert_eq!(branch_slug("x.lock"), "x-lock");
    assert_eq!(branch_slug("  Lead  Dev  "), "lead-dev");
}

#[test]
fn an_empty_or_non_ascii_name_becomes_agent() {
    assert_eq!(branch_slug(""), "agent");
    assert_eq!(branch_slug("日本語"), "agent");
    assert_eq!(branch_slug("---"), "agent");
}

#[test]
fn a_long_name_is_cut_to_forty_characters_without_a_trailing_separator() {
    let slug = branch_slug(&format!("{}-b", "a".repeat(39)));
    assert_eq!(slug, "a".repeat(39));
    assert_eq!(branch_slug(&"x".repeat(100)).len(), 40);
}

#[test]
fn names_that_differ_only_by_case_get_distinct_branches() {
    let first = free_branch_name("Bob", &[]);
    let second = free_branch_name("bob", std::slice::from_ref(&first));
    assert_eq!(first, "office-harness/bob");
    assert_eq!(second, "office-harness/bob-2");
}

#[test]
fn a_taken_name_gets_the_lowest_free_suffix() {
    let existing = vec![
        "office-harness/dev".to_owned(),
        "office-harness/dev-2".to_owned(),
        "office-harness/dev-4".to_owned(),
    ];
    assert_eq!(free_branch_name("dev", &existing), "office-harness/dev-3");
}

#[test]
fn every_made_name_passes_the_shape_check() {
    for name in ["a..b", "x@{1}", "x.lock", "日本語", "Bob", &"y".repeat(80)] {
        let branch = free_branch_name(name, &[free_branch_name(name, &[])]);
        assert!(is_office_branch_name(&branch), "{branch}");
    }
}

#[test]
fn the_shape_check_refuses_other_names() {
    for name in [
        "main",
        "office-harness/",
        "office-harness/A",
        "office-harness/a--b",
        "office-harness/-a",
        "office-harness/a/b",
        "office-harness/a.b",
        &format!("office-harness/{}", "a".repeat(41)),
    ] {
        assert!(!is_office_branch_name(name), "{name}");
    }
}

#[test]
fn a_commit_base_is_passed_as_the_id_and_a_branch_as_its_ref() {
    let commit = "a".repeat(40);
    assert_eq!(base_ref(&commit), commit);
    assert_eq!(base_ref("-x"), "refs/heads/-x");
    assert_eq!(base_ref("main"), "refs/heads/main");
    assert!(!is_commit_id(&"A".repeat(40)));
    assert!(is_commit_id(&"0".repeat(64)));
}

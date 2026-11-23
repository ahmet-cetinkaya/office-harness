// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn walks_children_and_grandchildren_only() {
    let children = HashMap::from([(10, vec![11, 13]), (11, vec![12]), (1, vec![10, 20])]);
    let mut found = walk_children(10, &children, &HashMap::new());
    found.sort_unstable();
    assert_eq!(found, [11, 12, 13]);
}

#[test]
fn a_loop_in_the_table_does_not_hold_the_walk() {
    let children = HashMap::from([(10, vec![11]), (11, vec![10, 12]), (12, vec![11])]);
    let mut found = walk_children(10, &children, &HashMap::new());
    found.sort_unstable();
    assert_eq!(found, [11, 12]);
}

#[test]
fn a_child_older_than_its_parent_and_its_subtree_are_not_listed() {
    // 12 holds a stale parent link to 11 (started at 50, before 11 at 100);
    // 13 hangs below 12 and 14 is a real child of 11.
    let children = HashMap::from([(10, vec![11]), (11, vec![12, 14]), (12, vec![13])]);
    let started_at = HashMap::from([(10, 90), (11, 100), (12, 50), (13, 120), (14, 100)]);

    let mut found = walk_children(10, &children, &started_at);

    found.sort_unstable();
    assert_eq!(found, [11, 14]);
}

#[test]
fn a_child_that_started_in_the_same_unit_as_its_parent_is_listed() {
    let children = HashMap::from([(10, vec![11])]);
    let started_at = HashMap::from([(10, 100), (11, 100)]);

    assert_eq!(walk_children(10, &children, &started_at), [11]);
}

#[test]
fn the_pid_of_the_server_and_init_cannot_be_opened() {
    let inspector = SystemProcessInspector::new();

    for pid in [0, 1] {
        assert_eq!(inspector.open(pid).err(), Some(ReadError::NotReadable));
    }
}

#[test]
fn stops_walking_at_the_cap() {
    let table_size = u32::try_from(MAX_DESCENDANTS).expect("the cap fits u32") + 50;
    let wide = HashMap::from([(1, (10..10 + table_size).collect())]);
    assert_eq!(
        walk_children(1, &wide, &HashMap::new()).len(),
        MAX_DESCENDANTS
    );
}

#[cfg(target_os = "linux")]
#[test]
fn the_sysinfo_reader_gives_the_command_line_form_of_the_proc_reader() {
    let pid = std::process::id();
    let from_proc = proc_reader::read_identity(pid).expect("this process is readable");
    let from_sysinfo = sysinfo_reader::read_identity(pid).expect("this process is readable");
    assert_eq!(from_proc.command_hash, from_sysinfo.command_hash);
    assert_eq!(from_proc.command_display, from_sysinfo.command_display);
}

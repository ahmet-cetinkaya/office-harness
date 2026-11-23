// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn walks_children_and_grandchildren_only() {
    let children = HashMap::from([(10, vec![11, 13]), (11, vec![12]), (1, vec![10, 20])]);
    let mut found = walk_children(10, &children);
    found.sort_unstable();
    assert_eq!(found, [11, 12, 13]);
}

#[test]
fn a_loop_in_the_table_does_not_hold_the_walk() {
    let children = HashMap::from([(10, vec![11]), (11, vec![10, 12]), (12, vec![11])]);
    let mut found = walk_children(10, &children);
    found.sort_unstable();
    assert_eq!(found, [11, 12]);
}

#[test]
fn stops_walking_at_the_cap() {
    let table_size = u32::try_from(MAX_DESCENDANTS).expect("the cap fits u32") + 50;
    let wide = HashMap::from([(1, (10..10 + table_size).collect())]);
    assert_eq!(walk_children(1, &wide).len(), MAX_DESCENDANTS);
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

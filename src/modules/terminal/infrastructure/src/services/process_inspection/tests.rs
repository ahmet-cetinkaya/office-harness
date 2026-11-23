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

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod pid_handle_exit {
    use terminal_application::common::models::ProcessIdentity;

    use super::super::pid_handle::exit_confirmed;
    use super::*;

    const OPENED_AT: u64 = 100;

    fn identity_started_at(started_at: u64) -> ProcessIdentity {
        ProcessIdentity::new(
            std::process::id(),
            started_at,
            "a command",
            "boot".to_owned(),
        )
    }

    #[test]
    fn a_process_with_the_same_start_time_has_not_exited() {
        assert!(!exit_confirmed(
            &Ok(identity_started_at(OPENED_AT)),
            OPENED_AT
        ));
    }

    #[test]
    fn an_id_that_names_another_process_counts_as_exited() {
        assert!(exit_confirmed(
            &Ok(identity_started_at(OPENED_AT + 1)),
            OPENED_AT
        ));
    }

    #[test]
    fn a_process_that_is_not_running_or_a_zombie_counts_as_exited() {
        assert!(exit_confirmed(&Err(ReadError::NotRunning), OPENED_AT));
    }

    #[test]
    fn a_process_that_cannot_be_read_may_still_run() {
        assert!(!exit_confirmed(&Err(ReadError::NotReadable), OPENED_AT));
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later

use std::process::Stdio;

use super::*;

/// The state letter of a process in `/proc`, or `None` when it is gone.
#[cfg(target_os = "linux")]
fn process_state(pid: u32) -> Option<char> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // The name sits in brackets and may hold spaces; the state follows it.
    stat.rsplit_once(") ")?.1.chars().next()
}

#[cfg(target_os = "linux")]
#[test]
fn an_exited_child_stays_unreaped_until_the_caller_takes_its_status() {
    let mut command = Command::new("sh");
    command
        .args(["-c", "exit 3"])
        .stdin(Stdio::null())
        .stdout(Stdio::null());
    own_process_group(&mut command);
    let mut child = command.spawn().expect("spawn");
    let pid = child.id();

    assert!(wait_until(
        &mut child,
        Instant::now() + Duration::from_secs(10)
    ));

    // A zombie still owns its pid, so the group signal of `end_tree` cannot
    // reach a process that took the pid after a reap.
    assert_eq!(process_state(pid), Some('Z'));
    assert_eq!(child.wait().expect("status").code(), Some(3));
    assert_eq!(process_state(pid), None);
}

#[cfg(unix)]
#[test]
fn a_child_that_outlives_the_deadline_is_not_reported_as_exited() {
    let mut command = Command::new("sleep");
    command.arg("30").stdin(Stdio::null());
    own_process_group(&mut command);
    let mut child = command.spawn().expect("spawn");

    assert!(!wait_until(
        &mut child,
        Instant::now() + Duration::from_millis(50)
    ));

    end_tree(&mut child);
}

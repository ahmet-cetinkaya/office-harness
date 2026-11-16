// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[cfg(not(target_os = "macos"))]
#[test]
fn a_path_with_a_space_is_quoted_whole() {
    let path = "/home/Jane Doe/.local/share/office-harness/bin/1.0.0/office-harnessd";

    let line = command_line_path(path).expect("a safe path");

    assert_eq!(line, format!("\"{path}\""));
}

#[cfg(not(target_os = "macos"))]
#[test]
fn a_path_with_a_quote_is_refused() {
    assert!(command_line_path("/home/a\"b/office-harnessd").is_err());
}

#[cfg(target_os = "linux")]
#[test]
fn a_path_that_the_unit_would_expand_is_refused() {
    for path in [
        "/home/a%h/office-harnessd",
        "/home/$USER/office-harnessd",
        "/home/a`b`/office-harnessd",
        "/home/a\\b/office-harnessd",
    ] {
        assert!(command_line_path(path).is_err(), "{path}");
    }
}

#[cfg(target_os = "macos")]
#[test]
fn a_macos_path_is_kept_as_is() {
    let path = "/Users/Jane Doe/Library/Application Support/office-harnessd";

    assert_eq!(command_line_path(path).expect("a path"), path);
}

#[cfg(target_os = "linux")]
#[test]
fn the_systemd_rule_stops_restarts_of_a_second_server() {
    let content = systemd_restart_rule::content();

    assert!(content.starts_with("[Service]\n"));
    assert!(content.contains(&format!(
        "RestartPreventExitStatus={}",
        crate::bootstrap::ALREADY_RUNNING_EXIT_CODE
    )));
}

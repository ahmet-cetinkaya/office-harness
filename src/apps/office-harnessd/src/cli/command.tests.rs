// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn arguments(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

#[test]
fn the_first_argument_names_the_subcommand() {
    let all = arguments(&["register", "--data-folder", "somewhere"]);

    let (command, rest) = split_command(&all).expect("a subcommand");

    assert_eq!(command, Command::Register);
    assert_eq!(rest, &all[1..]);
}

#[test]
fn each_subcommand_is_known() {
    for (name, expected) in [
        ("register", Command::Register),
        ("unregister", Command::Unregister),
        ("status", Command::Status),
    ] {
        let all = arguments(&[name]);

        assert_eq!(
            split_command(&all).map(|(command, _)| command),
            Some(expected)
        );
    }
}

#[test]
fn no_subcommand_means_the_program_serves() {
    assert!(split_command(&[]).is_none());
    assert!(split_command(&arguments(&["--data-folder", "somewhere"])).is_none());
}

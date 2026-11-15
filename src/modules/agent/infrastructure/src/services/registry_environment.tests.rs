// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

const OUTPUT: &str = "\r\nHKEY_CURRENT_USER\\Environment\r\n    Path    REG_EXPAND_SZ    %USERPROFILE%\\bin;C:\\tools\r\n    TEMP    REG_EXPAND_SZ    %USERPROFILE%\\AppData\\Local\\Temp\r\n    Count    REG_DWORD    0x1\r\n";

fn pair(name: &str, value: &str) -> (String, String) {
    (name.to_owned(), value.to_owned())
}

#[test]
fn parses_text_values_and_skips_other_types() {
    let entries = parse_registry_output(OUTPUT);
    assert_eq!(
        entries,
        [
            pair("Path", "%USERPROFILE%\\bin;C:\\tools"),
            pair("TEMP", "%USERPROFILE%\\AppData\\Local\\Temp"),
        ]
    );
}

#[test]
fn expands_known_names_and_keeps_unknown_ones() {
    let base = Environment::from([pair("USERPROFILE", "C:\\Users\\me")]);
    assert_eq!(
        expand("%userprofile%\\bin;%UNKNOWN%", &base),
        "C:\\Users\\me\\bin;%UNKNOWN%"
    );
    assert_eq!(expand("50%", &base), "50%");
}

#[test]
fn the_path_is_the_machine_entries_then_the_user_entries() {
    let base = Environment::from([pair("USERPROFILE", "C:\\Users\\me")]);
    let machine = [
        pair("Path", "C:\\Windows"),
        pair("ComSpec", "C:\\Windows\\cmd.exe"),
    ];
    let user = [pair("Path", "%USERPROFILE%\\bin")];
    let layered = layer_registry_environment(&base, &machine, &user);
    assert_eq!(
        layered.get("PATH").map(String::as_str),
        Some("C:\\Windows;C:\\Users\\me\\bin")
    );
    assert_eq!(
        layered.get("ComSpec").map(String::as_str),
        Some("C:\\Windows\\cmd.exe")
    );
}

// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn the_title_carries_the_count_only_when_an_agent_needs_the_user() {
    assert_eq!(window_title(0), "office-harness");
    assert_eq!(window_title(3), "office-harness (3)");
}

/// The `allow-*` permission that `build.rs` makes for a command name.
fn permission_for(command: &str) -> String {
    format!("allow-{}", command.replace('_', "-"))
}

/// The names in the `WEBVIEW_COMMANDS` list of `build.rs`.
fn webview_commands() -> Vec<String> {
    let source = include_str!("../build.rs");
    let list = source
        .split("WEBVIEW_COMMANDS: &[&str] = &[")
        .nth(1)
        .and_then(|rest| rest.split("];").next())
        .expect("the command list in build.rs");
    list.split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

#[test]
fn the_webview_capability_grants_exactly_the_listed_commands_and_notifications() {
    let capability: Value =
        serde_json::from_str(include_str!("../capabilities/default.json")).expect("valid json");
    let mut granted: Vec<String> = capability["permissions"]
        .as_array()
        .expect("a permission list")
        .iter()
        .map(|permission| permission.as_str().expect("a text").to_owned())
        .collect();
    let mut expected: Vec<String> = webview_commands()
        .iter()
        .map(|command| permission_for(command))
        .chain(["notification:default".to_owned()])
        .collect();
    granted.sort();
    expected.sort();
    assert_eq!(granted, expected);
    assert!(
        granted
            .iter()
            .all(|permission| !permission.starts_with("dialog:") && !permission.starts_with("fs:"))
    );
}

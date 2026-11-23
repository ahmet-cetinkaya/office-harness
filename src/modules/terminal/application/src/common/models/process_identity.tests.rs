// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

const BOOT_ID: &str = "3f1c9a52-0b7e-4c1d-9d57-6a1f0e2b8c44";

fn identity(started_at: u64, command_line: &str, boot: &str) -> ProcessIdentity {
    ProcessIdentity::new(7, started_at, command_line, boot.to_owned())
}

#[test]
fn the_same_values_match() {
    let stored = identity(10, "node server.js", BOOT_ID);
    assert_eq!(stored.mismatch(&stored.clone()), None);
}

#[test]
fn another_start_time_is_named() {
    let stored = identity(10, "node server.js", BOOT_ID);
    let live = identity(11, "node server.js", BOOT_ID);
    assert_eq!(stored.mismatch(&live), Some(Mismatch::OtherStartTime));
}

#[test]
fn another_command_line_is_named() {
    let stored = identity(10, "node server.js", BOOT_ID);
    let live = identity(10, "node other.js", BOOT_ID);
    assert_eq!(stored.mismatch(&live), Some(Mismatch::OtherCommandLine));
}

#[test]
fn another_boot_id_names_the_restart_before_any_other_value() {
    let stored = identity(10, "node server.js", BOOT_ID);
    let live = identity(11, "node other.js", "a-different-boot-id");
    assert_eq!(stored.mismatch(&live), Some(Mismatch::MachineRestarted));
}

#[test]
fn boot_times_within_the_tolerance_are_one_machine_run() {
    let stored = identity(10, "cmd", "1000");
    assert_eq!(stored.mismatch(&identity(10, "cmd", "1002")), None);
    assert_eq!(stored.mismatch(&identity(10, "cmd", "998")), None);
}

#[test]
fn boot_times_beyond_the_tolerance_are_named_last() {
    let stored = identity(10, "cmd", "1000");
    assert_eq!(
        stored.mismatch(&identity(10, "cmd", "1003")),
        Some(Mismatch::BootTimeChanged)
    );
    assert_eq!(
        stored.mismatch(&identity(11, "cmd", "1003")),
        Some(Mismatch::OtherStartTime)
    );
}

#[test]
fn the_hash_covers_the_whole_line_and_the_display_text_is_capped() {
    let prefix = "x".repeat(COMMAND_DISPLAY_LIMIT);
    let first = identity(1, &format!("{prefix}a"), BOOT_ID);
    let second = identity(1, &format!("{prefix}b"), BOOT_ID);
    assert_eq!(first.command_display, prefix);
    assert_eq!(first.command_display, second.command_display);
    assert_eq!(first.mismatch(&second), Some(Mismatch::OtherCommandLine));
    assert_eq!(
        first.command_hash,
        identity(1, &format!("{prefix}a"), BOOT_ID).command_hash
    );
}

#[test]
fn the_display_cap_counts_characters_not_bytes() {
    let line = "é".repeat(COMMAND_DISPLAY_LIMIT + 5);
    assert_eq!(
        identity(1, &line, BOOT_ID).command_display.chars().count(),
        COMMAND_DISPLAY_LIMIT
    );
}

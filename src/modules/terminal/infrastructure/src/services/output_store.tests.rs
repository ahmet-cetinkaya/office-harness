// SPDX-License-Identifier: AGPL-3.0-or-later

use tempfile::TempDir;

use super::*;

fn key() -> OutputKey {
    OutputKey::parse("session-1").expect("valid key")
}

fn saved(folder: &TempDir) -> String {
    let bytes = OutputStore::read_saved(folder.path(), &key())
        .expect("read")
        .expect("file exists");
    String::from_utf8(bytes).expect("text")
}

#[test]
fn keeps_everything_below_the_limits() {
    let folder = TempDir::new().expect("temp folder");
    let limits = OutputLimits::new(100, 10_000).expect("limits");
    let mut store = OutputStore::create(folder.path(), &key(), limits).expect("create");
    let outcome = store.append(b"one\ntwo\n").expect("append");
    assert!(!outcome.dropped_oldest);
    assert_eq!(saved(&folder), "one\ntwo\n");
}

#[test]
fn drops_the_oldest_lines_above_the_line_limit_and_reports_it() {
    let folder = TempDir::new().expect("temp folder");
    let limits = OutputLimits::new(10, 1_000_000).expect("limits");
    let mut store = OutputStore::create(folder.path(), &key(), limits).expect("create");
    let mut reported = false;
    for number in 0..25 {
        reported |= store
            .append(format!("line{number:02}\n").as_bytes())
            .expect("append")
            .dropped_oldest;
    }
    let text = saved(&folder);
    assert!(reported);
    assert!(!text.contains("line00"));
    assert!(text.contains("line24"));
    assert!(text.lines().count() <= 10);
}

#[test]
fn drops_the_oldest_bytes_above_the_byte_limit_without_newlines() {
    let folder = TempDir::new().expect("temp folder");
    let limits = OutputLimits::new(1000, 100).expect("limits");
    let mut store = OutputStore::create(folder.path(), &key(), limits).expect("create");
    let mut reported = false;
    for _ in 0..30 {
        reported |= store.append(&[b'x'; 10]).expect("append").dropped_oldest;
    }
    assert!(reported);
    assert!(saved(&folder).len() <= 100);
}

#[test]
fn appends_after_a_trim_reach_the_new_file() {
    let folder = TempDir::new().expect("temp folder");
    let limits = OutputLimits::new(3, 1_000_000).expect("limits");
    let mut store = OutputStore::create(folder.path(), &key(), limits).expect("create");
    for number in 0..10 {
        store
            .append(format!("l{number}\n").as_bytes())
            .expect("append");
    }
    store.append(b"last\n").expect("append");
    assert!(saved(&folder).ends_with("last\n"));
}

#[test]
fn a_missing_file_reads_as_none() {
    let folder = TempDir::new().expect("temp folder");
    assert!(
        OutputStore::read_saved(folder.path(), &key())
            .expect("read")
            .is_none()
    );
}

#[cfg(unix)]
#[test]
fn the_file_is_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let folder = TempDir::new().expect("temp folder");
    let limits = OutputLimits::new(10, 1000).expect("limits");
    OutputStore::create(folder.path(), &key(), limits).expect("create");
    let mode = fs::metadata(folder.path().join("session-1.log"))
        .expect("metadata")
        .permissions()
        .mode();
    assert_eq!(mode & 0o077, 0);
}

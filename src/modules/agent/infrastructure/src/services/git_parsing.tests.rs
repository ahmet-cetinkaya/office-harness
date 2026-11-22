// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn record(kind: ChangeKind, path: &str, old_path: Option<&str>) -> ChangeRecord {
    ChangeRecord {
        kind,
        path: path.to_owned(),
        old_path: old_path.map(str::to_owned),
    }
}

#[test]
fn every_status_form_is_read() {
    let output = b"# branch.oid abc\0\
        1 .M N... 100644 100644 100644 aaa bbb dir/a b.txt\0\
        1 A. N... 000000 100644 100644 000 bbb new.txt\0\
        1 .D N... 100644 100644 000000 aaa aaa gone.txt\0\
        1 .T N... 100644 100644 120000 aaa aaa linked\0\
        1 .M S.M. 160000 160000 160000 aaa aaa module\0\
        2 R. N... 100644 100644 100644 aaa bbb R100 new name.txt\0old name.txt\0\
        u UU N... 100644 100644 100644 100644 a b c both.txt\0\
        ? untracked file.txt\0\
        ! ignored.log\0";

    assert_eq!(
        parse_status(output),
        [
            record(ChangeKind::Modified, "dir/a b.txt", None),
            record(ChangeKind::Added, "new.txt", None),
            record(ChangeKind::Deleted, "gone.txt", None),
            record(ChangeKind::TypeChanged, "linked", None),
            record(ChangeKind::Submodule, "module", None),
            record(ChangeKind::Renamed, "new name.txt", Some("old name.txt")),
            record(ChangeKind::Conflicted, "both.txt", None),
            record(ChangeKind::Untracked, "untracked file.txt", None),
        ]
    );
}

#[test]
fn a_staged_add_that_is_modified_after_is_still_added() {
    let output = b"1 AM N... 000000 100644 100644 000 bbb new.txt\0";
    assert_eq!(parse_status(output)[0].kind, ChangeKind::Added);
}

#[test]
fn name_status_reads_plain_and_renamed_records() {
    let output = b"M\0a.txt\0A\0b.txt\0D\0c.txt\0T\0d\0R100\0old.txt\0new.txt\0";

    assert_eq!(
        parse_name_status(output),
        [
            record(ChangeKind::Modified, "a.txt", None),
            record(ChangeKind::Added, "b.txt", None),
            record(ChangeKind::Deleted, "c.txt", None),
            record(ChangeKind::TypeChanged, "d", None),
            record(ChangeKind::Renamed, "new.txt", Some("old.txt")),
        ]
    );
}

#[test]
fn log_records_are_read_with_a_subject_that_holds_the_field_text() {
    let output = "aaaa\u{1f}aa\u{1f}Ana\u{1f}1700000000\u{1f}first\u{1e}\nbbbb\u{1f}bb\u{1f}Bo\u{1f}1700000001\u{1f}second one\u{1e}\n";

    let records = parse_log(output.as_bytes());

    assert_eq!(records.len(), 2);
    assert_eq!(records[0].id, "aaaa");
    assert_eq!(records[0].author_seconds, 1_700_000_000);
    assert_eq!(records[1].subject, "second one");
    assert_eq!(parse_log(b""), Vec::new());
}

#[test]
fn the_diff_body_starts_at_the_first_hunk() {
    let output = b"diff --git a/x b/x\nindex 1..2 100644\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n";

    let DiffBody::Text(start) = diff_body(output) else {
        panic!("a text diff");
    };

    assert!(output[start..].starts_with(b"@@ -1 +1 @@\n"));
}

#[test]
fn binary_and_empty_diffs_are_told_apart() {
    assert_eq!(
        diff_body(b"diff --git a/x b/x\nBinary files a/x and b/x differ\n"),
        DiffBody::Binary
    );
    assert_eq!(diff_body(b""), DiffBody::Empty);
    assert_eq!(
        diff_body(b"diff --git a/x b/y\nsimilarity index 100%\nrename from x\nrename to y\n"),
        DiffBody::Empty
    );
}

#[test]
fn an_untracked_file_diff_adds_every_line() {
    assert_eq!(
        added_file_diff(b"one\ntwo\n"),
        b"@@ -0,0 +1,2 @@\n+one\n+two\n"
    );
    assert_eq!(added_file_diff(b"one\n"), b"@@ -0,0 +1 @@\n+one\n");
    assert_eq!(
        added_file_diff(b"one\ntwo"),
        b"@@ -0,0 +1,2 @@\n+one\n+two\n\\ No newline at end of file\n"
    );
    assert_eq!(added_file_diff(b""), Vec::<u8>::new());
    assert_eq!(added_file_diff(b"\n"), b"@@ -0,0 +1 @@\n+\n");
}

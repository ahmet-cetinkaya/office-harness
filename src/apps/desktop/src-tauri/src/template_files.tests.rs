// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[allow(clippy::cast_possible_truncation)]
const CAP: usize = MAX_IMPORT_BYTES as usize;

fn file_with(bytes: &[u8]) -> (tempfile::TempDir, std::path::PathBuf) {
    let folder = tempfile::tempdir().expect("temp folder");
    let path = folder.path().join("templates.json");
    fs::write(&path, bytes).expect("write file");
    (folder, path)
}

#[test]
fn a_file_of_exactly_the_cap_is_read() {
    let (_folder, path) = file_with(&vec![b'a'; CAP]);
    assert_eq!(read_import_file(&path).expect("read").len(), CAP);
}

#[test]
fn a_file_one_byte_over_the_cap_is_refused() {
    let (_folder, path) = file_with(&vec![b'a'; CAP + 1]);
    assert!(matches!(
        read_import_file(&path),
        Err(TemplateFileError::TooLarge)
    ));
}

#[test]
fn a_folder_is_refused_as_not_a_regular_file() {
    let folder = tempfile::tempdir().expect("temp folder");
    assert!(matches!(
        read_import_file(folder.path()),
        Err(TemplateFileError::NotRegularFile)
    ));
}

#[cfg(unix)]
#[test]
fn a_fifo_is_refused_without_being_opened() {
    let folder = tempfile::tempdir().expect("temp folder");
    let path = folder.path().join("pipe");
    let made = std::process::Command::new("mkfifo")
        .arg(&path)
        .status()
        .expect("run mkfifo");
    assert!(made.success());
    assert!(matches!(
        read_import_file(&path),
        Err(TemplateFileError::NotRegularFile)
    ));
}

#[test]
fn bytes_that_are_not_text_are_refused() {
    let (_folder, path) = file_with(&[0xff, 0xfe]);
    assert!(matches!(
        read_import_file(&path),
        Err(TemplateFileError::NotText)
    ));
}

#[test]
fn a_missing_file_names_the_read_failure() {
    let folder = tempfile::tempdir().expect("temp folder");
    let error = read_import_file(&folder.path().join("gone.json")).expect_err("missing");
    assert!(error.to_string().starts_with("the file was not read: "));
}

#[test]
fn an_export_writes_the_contents_to_the_picked_path() {
    let folder = tempfile::tempdir().expect("temp folder");
    let path = folder.path().join("out.json");
    write_export_file(&path, "{}").expect("write");
    assert_eq!(fs::read_to_string(path).expect("read back"), "{}");
}

#[test]
fn a_failed_write_says_the_file_was_not_written() {
    let folder = tempfile::tempdir().expect("temp folder");
    let error = write_export_file(&folder.path().join("no-such-folder/out.json"), "{}")
        .expect_err("no folder");
    assert!(error.to_string().starts_with("the file was not written: "));
}

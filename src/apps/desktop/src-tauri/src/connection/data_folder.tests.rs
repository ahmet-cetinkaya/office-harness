// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs;

use tempfile::{TempDir, tempdir};

use super::*;

fn folder_with(files: &[(&str, &str)]) -> (TempDir, DataFolder) {
    let directory = tempdir().expect("temp folder");
    for (name, text) in files {
        fs::write(directory.path().join(name), text).expect("file");
    }
    let folder = DataFolder::at(directory.path().to_path_buf());
    (directory, folder)
}

#[test]
fn the_token_is_trimmed() {
    let (_directory, folder) = folder_with(&[("token", "abc123\n")]);

    assert_eq!(folder.read_token().expect("token"), "abc123");
}

#[test]
fn an_empty_token_file_is_an_error() {
    let (_directory, folder) = folder_with(&[("token", " \n")]);

    assert!(matches!(folder.read_token(), Err(ReadError::EmptyToken)));
}

#[test]
fn a_missing_token_file_is_an_error() {
    let (_directory, folder) = folder_with(&[]);

    assert!(matches!(folder.read_token(), Err(ReadError::Io(_))));
}

#[test]
fn reads_the_server_file() {
    let (_directory, folder) = folder_with(&[(
        "server.json",
        r#"{"port":4123,"protocol":1,"version":"0.1.0"}"#,
    )]);

    assert_eq!(folder.read_server_file().expect("file").port, 4123);
}

#[test]
fn a_missing_or_broken_server_file_is_an_error() {
    let (_missing, missing) = folder_with(&[]);
    let (_broken, broken) = folder_with(&[("server.json", "{")]);

    assert!(matches!(missing.read_server_file(), Err(ReadError::Io(_))));
    assert!(matches!(
        broken.read_server_file(),
        Err(ReadError::Invalid(_))
    ));
}

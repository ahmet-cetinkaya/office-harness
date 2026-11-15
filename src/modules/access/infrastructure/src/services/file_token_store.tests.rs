// SPDX-License-Identifier: AGPL-3.0-or-later

use access_domain::errors::TokenError;

use super::*;

fn token_path(folder: &tempfile::TempDir) -> PathBuf {
    folder.path().join("token")
}

fn secret_of(store: &FileTokenStore) -> String {
    store
        .current()
        .expect("current")
        .token
        .expose_secret()
        .to_owned()
}

#[test]
fn a_missing_file_gets_a_new_token_that_the_next_open_reads_back() {
    let folder = tempfile::tempdir().expect("folder");

    let first = FileTokenStore::open(token_path(&folder)).expect("create");
    let second = FileTokenStore::open(token_path(&folder)).expect("read");

    assert_eq!(secret_of(&first).len(), 64);
    assert_eq!(secret_of(&first), secret_of(&second));
    assert_eq!(first.current().expect("current").generation, 1);
}

#[test]
fn regeneration_replaces_the_file_and_raises_the_generation() {
    let folder = tempfile::tempdir().expect("folder");
    let store = FileTokenStore::open(token_path(&folder)).expect("open");
    let before = secret_of(&store);

    let snapshot = store.regenerate().expect("regenerate");

    assert_eq!(snapshot.generation, 2);
    assert_ne!(snapshot.token.expose_secret(), before);
    assert_eq!(
        std::fs::read_to_string(token_path(&folder)).expect("file"),
        snapshot.token.expose_secret()
    );
}

#[test]
fn subscribers_see_each_new_generation() {
    let folder = tempfile::tempdir().expect("folder");
    let store = FileTokenStore::open(token_path(&folder)).expect("open");
    let generations = store.subscribe();
    assert_eq!(*generations.borrow(), 1);

    store.regenerate().expect("regenerate");

    assert_eq!(*generations.borrow(), 2);
}

#[test]
fn a_file_that_is_not_a_token_is_refused_without_echoing_it() {
    let folder = tempfile::tempdir().expect("folder");
    std::fs::write(token_path(&folder), "not-a-token-but-secret-looking").expect("write");

    let error = FileTokenStore::open(token_path(&folder))
        .err()
        .expect("refused");

    assert!(matches!(
        error,
        AccessError::InvalidToken(TokenError::Malformed)
    ));
    assert!(!error.to_string().contains("secret-looking"));
    assert_eq!(
        std::fs::read_to_string(token_path(&folder)).expect("file"),
        "not-a-token-but-secret-looking",
        "the file is left as it was"
    );
}

#[cfg(unix)]
#[test]
fn the_token_file_is_owner_only_after_creation_and_after_regeneration() {
    use std::os::unix::fs::PermissionsExt;

    let folder = tempfile::tempdir().expect("folder");
    let mode = || {
        std::fs::metadata(token_path(&folder))
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777
    };
    let store = FileTokenStore::open(token_path(&folder)).expect("open");
    assert_eq!(mode(), 0o600);

    store.regenerate().expect("regenerate");

    assert_eq!(mode(), 0o600);
}

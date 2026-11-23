// SPDX-License-Identifier: AGPL-3.0-or-later

use secret_domain::LockedReason;

use super::*;

fn vault(folder: &tempfile::TempDir) -> KeyFileVault {
    KeyFileVault::new(folder.path().join("secrets.key"))
}

#[test]
fn a_stored_key_loads_back_and_is_gone_after_a_remove() {
    let folder = tempfile::tempdir().expect("folder");
    let vault = vault(&folder);
    let key = StoreKey::from_bytes([5; 32]);

    vault.store(&key).expect("store");
    assert!(vault.load().expect("load").expect("key").matches(&key));

    vault.remove().expect("remove");
    assert!(vault.load().expect("load").is_none());
    vault.remove().expect("a second remove is not an error");
}

#[test]
fn a_missing_file_is_locked_and_never_a_new_key() {
    let folder = tempfile::tempdir().expect("folder");
    let vault = vault(&folder);

    assert_eq!(
        secret_application::load_store_key(&vault).expect_err("locked"),
        VaultError::Locked(LockedReason::KeyFileMissing)
    );
    assert!(!folder.path().join("secrets.key").exists());
}

#[test]
fn a_file_of_the_wrong_size_is_damaged() {
    let folder = tempfile::tempdir().expect("folder");
    fs::write(folder.path().join("secrets.key"), b"short").expect("write");

    let error = vault(&folder).load().expect_err("damaged");

    assert_eq!(
        error,
        VaultError::Failed("the key file is damaged".to_owned())
    );
}

#[cfg(unix)]
#[test]
fn the_key_file_is_readable_by_its_owner_only() {
    use std::os::unix::fs::PermissionsExt;

    let folder = tempfile::tempdir().expect("folder");
    let vault = vault(&folder);
    vault.store(&StoreKey::from_bytes([5; 32])).expect("store");

    let mode = fs::metadata(folder.path().join("secrets.key"))
        .expect("metadata")
        .permissions()
        .mode();

    assert_eq!(mode & 0o777, 0o600);
}

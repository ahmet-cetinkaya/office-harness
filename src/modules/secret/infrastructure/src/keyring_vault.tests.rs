// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

/// Not the account of a real office, so a test never touches a real key.
const TEST_ACCOUNT: &str = "secret-store-key-test-0026";

/// A real keyring smoke test. It runs where the OS offers a keyring to this
/// process and says why when it skips (a headless Linux machine has none).
#[test]
fn a_key_round_trips_through_the_os_keyring_where_one_exists() {
    let vault = KeyringVault {
        account: TEST_ACCOUNT.to_owned(),
    };
    let key = StoreKey::from_bytes([3; 32]);
    if let Err(error) = vault.store(&key) {
        eprintln!("skipped: no usable OS keyring here ({error})");
        return;
    }

    let loaded = vault.load().expect("load").expect("key");
    vault.remove().expect("remove");

    assert!(loaded.matches(&key));
    assert!(vault.load().expect("load after remove").is_none());
}

#[test]
fn an_office_keeps_its_account_and_two_offices_differ() {
    let first = tempfile::tempdir().expect("folder");
    let second = tempfile::tempdir().expect("folder");

    let account = |folder: &tempfile::TempDir| {
        KeyringVault::for_office(&folder.path().join("id"))
            .expect("vault")
            .account
    };

    assert_eq!(account(&first), account(&first));
    assert_ne!(account(&first), account(&second));
    assert!(account(&first).starts_with(ACCOUNT_PREFIX));
}

#[test]
fn a_damaged_id_file_is_an_error_and_never_a_new_account() {
    let folder = tempfile::tempdir().expect("folder");
    let path = folder.path().join("id");
    fs::write(&path, "not an id").expect("write");

    assert!(matches!(
        KeyringVault::for_office(&path),
        Err(VaultError::Failed(_))
    ));
    assert_eq!(fs::read_to_string(&path).expect("read"), "not an id");
}

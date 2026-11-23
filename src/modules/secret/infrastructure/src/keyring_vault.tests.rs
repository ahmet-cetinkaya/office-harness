// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

/// Not the account of a real office, so a test never touches a real key.
const TEST_ACCOUNT: &str = "secret-store-key-test-0026";

/// A real keyring smoke test. It runs where the OS offers a keyring to this
/// process and says why when it skips (a headless Linux machine has none).
#[test]
fn a_key_round_trips_through_the_os_keyring_where_one_exists() {
    let vault = KeyringVault {
        account: TEST_ACCOUNT,
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

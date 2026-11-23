// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Mutex;

use super::*;

/// A vault in memory that can be told to fail or to lose what it stores.
struct FakeVault {
    custody: Custody,
    held: Mutex<Option<[u8; 32]>>,
    refuses_store: bool,
    forgets: bool,
}

impl FakeVault {
    fn new(custody: Custody) -> Self {
        Self {
            custody,
            held: Mutex::new(None),
            refuses_store: false,
            forgets: false,
        }
    }

    fn holds_a_key(&self) -> bool {
        self.held.lock().expect("lock").is_some()
    }
}

impl StoreKeyVault for FakeVault {
    fn custody(&self) -> Custody {
        self.custody
    }

    fn load(&self) -> Result<Option<StoreKey>, VaultError> {
        let held = *self.held.lock().expect("lock");
        Ok(held.map(StoreKey::from_bytes))
    }

    fn store(&self, key: &StoreKey) -> Result<(), VaultError> {
        if self.refuses_store {
            return Err(VaultError::Locked(LockedReason::KeyringNotRunning));
        }
        if !self.forgets {
            *self.held.lock().expect("lock") = Some(*key.expose());
        }
        Ok(())
    }

    fn remove(&self) -> Result<(), VaultError> {
        *self.held.lock().expect("lock") = None;
        Ok(())
    }
}

fn key() -> StoreKey {
    StoreKey::from_bytes([9; 32])
}

#[test]
fn a_working_keyring_keeps_the_key_and_the_file_stays_empty() {
    let keyring = FakeVault::new(Custody::OsKeyring);
    let file = FakeVault::new(Custody::KeyFile);

    let custody = provision_store_key(&keyring, &file, &key()).expect("custody");

    assert_eq!(custody, Custody::OsKeyring);
    assert!(keyring.holds_a_key());
    assert!(!file.holds_a_key());
}

#[test]
fn a_keyring_that_refuses_the_key_sends_it_to_the_file() {
    let mut keyring = FakeVault::new(Custody::OsKeyring);
    keyring.refuses_store = true;
    let file = FakeVault::new(Custody::KeyFile);

    let custody = provision_store_key(&keyring, &file, &key()).expect("custody");

    assert_eq!(custody, Custody::KeyFile);
    assert!(file.holds_a_key());
}

#[test]
fn a_keyring_that_loses_the_key_fails_the_probe_and_the_file_takes_it() {
    let mut keyring = FakeVault::new(Custody::OsKeyring);
    keyring.forgets = true;
    let file = FakeVault::new(Custody::KeyFile);

    let custody = provision_store_key(&keyring, &file, &key()).expect("custody");

    assert_eq!(custody, Custody::KeyFile);
    assert!(!keyring.holds_a_key());
}

#[test]
fn a_vault_with_no_key_is_locked_with_the_reason_of_its_custody() {
    assert_eq!(
        load_store_key(&FakeVault::new(Custody::OsKeyring)).expect_err("locked"),
        VaultError::Locked(LockedReason::KeyringEntryMissing)
    );
    assert_eq!(
        load_store_key(&FakeVault::new(Custody::KeyFile)).expect_err("locked"),
        VaultError::Locked(LockedReason::KeyFileMissing)
    );
}

#[test]
fn a_stored_key_loads() {
    let file = FakeVault::new(Custody::KeyFile);
    file.store(&key()).expect("store");

    assert!(load_store_key(&file).expect("key").matches(&key()));
}

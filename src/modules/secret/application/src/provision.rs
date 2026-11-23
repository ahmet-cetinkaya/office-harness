// SPDX-License-Identifier: AGPL-3.0-or-later

use secret_domain::{Custody, LockedReason, StoreKey};

use crate::ports::{StoreKeyVault, VaultError};

/// Keeps a new store key in the keyring when the keyring takes it and gives
/// it back unchanged, and otherwise in the key file (FR-003). The caller
/// stores the returned custody; the choice is never made again.
///
/// # Errors
/// The key file vault's error when the keyring failed and the file failed too.
pub fn provision_store_key(
    keyring: &dyn StoreKeyVault,
    key_file: &dyn StoreKeyVault,
    key: &StoreKey,
) -> Result<Custody, VaultError> {
    if keyring_keeps(keyring, key) {
        return Ok(keyring.custody());
    }
    key_file.store(key)?;
    Ok(key_file.custody())
}

/// The write then read-back probe: a keyring that accepts a key and loses it
/// is no custody. A keyring that already holds a key for this office has an
/// owner that is not this store, so it is never written over (the key file
/// takes the new key instead).
fn keyring_keeps(keyring: &dyn StoreKeyVault, key: &StoreKey) -> bool {
    if !matches!(keyring.load(), Ok(None)) {
        return false;
    }
    if keyring.store(key).is_err() {
        return false;
    }
    let kept = matches!(keyring.load(), Ok(Some(stored)) if stored.matches(key));
    if !kept {
        // Best effort: an entry that did not read back must not stay behind.
        let _ = keyring.remove();
    }
    kept
}

/// The store key of an office that already has sealed secrets.
///
/// # Errors
/// `Locked` with the reason when the vault answers with no key (the entry or
/// the file is missing) or cannot answer; the vault's own error otherwise.
pub fn load_store_key(vault: &dyn StoreKeyVault) -> Result<StoreKey, VaultError> {
    vault
        .load()?
        .ok_or(VaultError::Locked(match vault.custody() {
            Custody::OsKeyring => LockedReason::KeyringEntryMissing,
            Custody::KeyFile => LockedReason::KeyFileMissing,
        }))
}

#[cfg(test)]
#[path = "provision.tests.rs"]
mod tests;

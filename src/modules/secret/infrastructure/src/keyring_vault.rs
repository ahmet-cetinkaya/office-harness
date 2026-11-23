// SPDX-License-Identifier: AGPL-3.0-or-later

use keyring::{Entry, Error};
use secret_application::{StoreKeyVault, VaultError};
use secret_domain::{Custody, LockedReason, StoreKey};
use zeroize::Zeroizing;

const SERVICE: &str = "office-harness";
const ACCOUNT: &str = "secret-store-key";

/// The store key in the OS keyring: Keychain on macOS, Credential Manager on
/// Windows, Secret Service on Linux. The item belongs to the OS user.
#[derive(Debug)]
pub struct KeyringVault {
    account: &'static str,
}

impl Default for KeyringVault {
    fn default() -> Self {
        Self { account: ACCOUNT }
    }
}

impl KeyringVault {
    fn entry(&self) -> Result<Entry, VaultError> {
        Entry::new(SERVICE, self.account).map_err(|error| translate(&error))
    }
}

impl StoreKeyVault for KeyringVault {
    fn custody(&self) -> Custody {
        Custody::OsKeyring
    }

    fn load(&self) -> Result<Option<StoreKey>, VaultError> {
        match self.entry()?.get_secret() {
            Ok(bytes) => {
                let bytes = Zeroizing::new(bytes);
                StoreKey::from_slice(&bytes)
                    .map(Some)
                    .ok_or_else(|| VaultError::Failed("the keyring entry is damaged".to_owned()))
            }
            Err(Error::NoEntry) => Ok(None),
            Err(error) => Err(translate(&error)),
        }
    }

    fn store(&self, key: &StoreKey) -> Result<(), VaultError> {
        self.entry()?
            .set_secret(key.expose())
            .map_err(|error| translate(&error))
    }

    fn remove(&self) -> Result<(), VaultError> {
        match self.entry()?.delete_credential() {
            Ok(()) | Err(Error::NoEntry) => Ok(()),
            Err(error) => Err(translate(&error)),
        }
    }
}

/// Names the reason without the platform's own message, which is for the
/// log of the platform and can hold item names.
fn translate(error: &Error) -> VaultError {
    match error {
        Error::NoStorageAccess(_) => VaultError::Locked(LockedReason::KeyringLocked),
        Error::NoDefaultStore | Error::PlatformFailure(_) => {
            VaultError::Locked(LockedReason::KeyringNotRunning)
        }
        _ => VaultError::Failed("the keyring refused the request".to_owned()),
    }
}

/// A keyring that is not there: it takes no key, so the store key goes to the
/// key file. For a machine that must not touch the OS keyring, and for tests.
#[derive(Debug, Default)]
pub struct UnavailableKeyring;

impl StoreKeyVault for UnavailableKeyring {
    fn custody(&self) -> Custody {
        Custody::OsKeyring
    }

    fn load(&self) -> Result<Option<StoreKey>, VaultError> {
        Err(VaultError::Locked(LockedReason::KeyringNotRunning))
    }

    fn store(&self, _key: &StoreKey) -> Result<(), VaultError> {
        Err(VaultError::Locked(LockedReason::KeyringNotRunning))
    }

    fn remove(&self) -> Result<(), VaultError> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "keyring_vault.tests.rs"]
mod tests;

// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs;
use std::io;
use std::path::Path;

use keyring::{Entry, Error};
use private_fs::replace_private_file;
use secret_application::{StoreKeyVault, VaultError};
use secret_domain::{Custody, LockedReason, StoreKey};
use zeroize::Zeroizing;

const SERVICE: &str = "office-harness";
const ACCOUNT_PREFIX: &str = "secret-store-key-";
const OFFICE_ID_BYTES: usize = 16;

/// The store key in the OS keyring: Keychain on macOS, Credential Manager on
/// Windows, Secret Service on Linux. The item belongs to the OS user, so each
/// office has its own account: two offices of one user never read, overwrite,
/// or remove each other's key.
#[derive(Debug)]
pub struct KeyringVault {
    account: String,
}

impl KeyringVault {
    /// The vault of the office whose random id is kept in `id_path`, in its
    /// data folder. The file is made at the first call, so the account moves
    /// with the data folder.
    ///
    /// # Errors
    /// `Failed` when the id file cannot be read, made, or is damaged.
    pub fn for_office(id_path: &Path) -> Result<Self, VaultError> {
        let id = office_id(id_path).map_err(|error| {
            VaultError::Failed(format!(
                "the office id file could not be used ({:?})",
                error.kind()
            ))
        })?;
        Ok(Self {
            account: format!("{ACCOUNT_PREFIX}{id}"),
        })
    }

    fn entry(&self) -> Result<Entry, VaultError> {
        Entry::new(SERVICE, &self.account).map_err(|error| translate(&error))
    }
}

/// The id in the file, or a new random one that is written first.
fn office_id(path: &Path) -> io::Result<String> {
    match fs::read_to_string(path) {
        Ok(text) => {
            let id = text.trim();
            if is_office_id(id) {
                Ok(id.to_owned())
            } else {
                Err(io::Error::new(io::ErrorKind::InvalidData, "damaged id"))
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let mut bytes = [0_u8; OFFICE_ID_BYTES];
            getrandom::fill(&mut bytes).map_err(io::Error::other)?;
            let id = format!("{:032x}", u128::from_be_bytes(bytes));
            replace_private_file(path, id.as_bytes())?;
            Ok(id)
        }
        Err(error) => Err(error),
    }
}

fn is_office_id(text: &str) -> bool {
    text.len() == OFFICE_ID_BYTES * 2 && text.bytes().all(|byte| byte.is_ascii_hexdigit())
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

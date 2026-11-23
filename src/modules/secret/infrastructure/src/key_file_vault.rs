// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs;
use std::io;
use std::path::PathBuf;

use private_fs::replace_private_file;
use secret_application::{StoreKeyVault, VaultError};
use secret_domain::{Custody, StoreKey};
use zeroize::Zeroizing;

/// The store key in one owner-only file of the office's private folder (the
/// 0001/FR-009 rule). It is raw key bytes, 32 of them.
#[derive(Debug)]
pub struct KeyFileVault {
    path: PathBuf,
}

impl KeyFileVault {
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl StoreKeyVault for KeyFileVault {
    fn custody(&self) -> Custody {
        Custody::KeyFile
    }

    fn load(&self) -> Result<Option<StoreKey>, VaultError> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => Zeroizing::new(bytes),
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(failure(&error)),
        };
        StoreKey::from_slice(&bytes)
            .map(Some)
            .ok_or_else(|| VaultError::Failed("the key file is damaged".to_owned()))
    }

    fn store(&self, key: &StoreKey) -> Result<(), VaultError> {
        replace_private_file(&self.path, key.expose()).map_err(|error| failure(&error))
    }

    fn remove(&self) -> Result<(), VaultError> {
        match fs::remove_file(&self.path) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(failure(&error)),
            _ => Ok(()),
        }
    }
}

/// The error kind only: an I/O error text can name a path, never a key.
fn failure(error: &io::Error) -> VaultError {
    VaultError::Failed(format!(
        "the key file could not be used ({:?})",
        error.kind()
    ))
}

#[cfg(test)]
#[path = "key_file_vault.tests.rs"]
mod tests;

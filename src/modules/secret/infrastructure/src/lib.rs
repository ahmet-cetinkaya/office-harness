// SPDX-License-Identifier: AGPL-3.0-or-later

//! Secret module infrastructure (0026): the sealer, the key file vault, and
//! the OS keyring vault.

mod chacha_sealer;
mod key_file_vault;
mod keyring_vault;
pub mod persistence;
mod store_key_generator;

pub use chacha_sealer::ChaChaRecordSealer;
pub use key_file_vault::KeyFileVault;
pub use keyring_vault::KeyringVault;
pub use store_key_generator::{OsStoreKeyGenerator, generate_store_key};

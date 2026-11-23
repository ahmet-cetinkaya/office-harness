// SPDX-License-Identifier: AGPL-3.0-or-later

//! Secret module application (0026): the ports of the store key vault and
//! the record sealer, and the rules that provision and load the store key.

mod ports;
mod provision;

pub use ports::{RecordSealer, SealError, StoreKeyVault, VaultError};
pub use provision::{load_store_key, provision_store_key};

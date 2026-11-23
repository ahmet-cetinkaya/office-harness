// SPDX-License-Identifier: AGPL-3.0-or-later

//! Secret module application (0026): the ports, the rules that provision and
//! load the store key, and the service of the secret requests.

mod errors;
mod models;
mod ports;
mod provision;
mod redactor;
mod service;
mod store_ports;

pub use errors::SecretServiceError;
pub use models::{AvailableSecret, GrantSummary, ReadAnswer, SecretSummary, StoreState};
pub use ports::{RecordSealer, SealError, StoreKeyVault, VaultError};
pub use provision::{load_store_key, provision_store_key};
pub use redactor::SecretRedactor;
pub use service::{SecretPorts, SecretService};
pub use store_ports::{
    AgentDirectory, SecretClock, SecretStore, StoreError, StoreKeyGenerator, StoredGrant,
    StoredSecret,
};

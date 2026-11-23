// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use agent_application::common::interfaces::AgentStore;
use agent_infrastructure::persistence::SharedConnection;
use secret_application::{SecretPorts, SecretRedactor, SecretService, StoreKeyVault};
use secret_infrastructure::persistence::SecretRepository;
use secret_infrastructure::{
    ChaChaRecordSealer, KeyFileVault, KeyringVault, OsStoreKeyGenerator, UnavailableKeyring,
};

use crate::bridges::{AgentDirectoryBridge, SystemClock};
use crate::config::Config;

/// Cleared by tests so that a test office never writes to the developer's own
/// OS keyring.
static OS_KEYRING_ENABLED: AtomicBool = AtomicBool::new(true);

/// Makes every office that starts after this call keep its store key in the
/// key file only. It is for tests, which must not touch the real keyring.
pub fn disable_os_keyring() {
    OS_KEYRING_ENABLED.store(false, Ordering::SeqCst);
}

/// The secret services of the host (0026): the one service the gateway and
/// the agent connection call.
pub struct SecretServices {
    pub service: Arc<SecretService>,
    /// Knows the secret values for the redaction (FR-012); the gateway keeps
    /// it current at each change.
    pub redactor: Arc<SecretRedactor>,
}

/// Builds the secret service over the one database connection. The store key
/// goes to the OS keyring when it takes it, and otherwise to the owner-only
/// key file in the office's data folder (FR-003).
#[must_use]
pub fn build_secret_services(
    connection: &SharedConnection,
    config: &Config,
    agents: Arc<dyn AgentStore>,
    redactor: Arc<SecretRedactor>,
) -> SecretServices {
    let service = SecretService::new(SecretPorts {
        store: Arc::new(SecretRepository::new(connection.handle())),
        keyring: keyring(),
        key_file: Arc::new(KeyFileVault::new(config.secret_key_path())),
        generator: Arc::new(OsStoreKeyGenerator),
        sealer: Arc::new(ChaChaRecordSealer),
        agents: Arc::new(AgentDirectoryBridge::new(agents)),
        clock: Arc::new(SystemClock),
    });
    SecretServices {
        service: Arc::new(service),
        redactor,
    }
}

fn keyring() -> Arc<dyn StoreKeyVault> {
    if OS_KEYRING_ENABLED.load(Ordering::SeqCst) {
        Arc::new(KeyringVault::default())
    } else {
        Arc::new(UnavailableKeyring)
    }
}

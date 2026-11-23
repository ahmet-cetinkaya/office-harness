// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! The OS keyring is reached from the server's own async runtime (0026): while
//! the server is built, on an async thread, and from the blocking threads of a
//! request. Only a read is made: a test never writes the entry of a real
//! office. Where the OS offers no keyring the test says so and passes.

use secret_application::StoreKeyVault;
use secret_infrastructure::KeyringVault;

/// A real read of an account that no office uses.
fn vault_of_a_test_office() -> KeyringVault {
    let folder = tempfile::tempdir().expect("folder");
    KeyringVault::for_office(&folder.path().join("id")).expect("vault")
}

#[tokio::test(flavor = "multi_thread")]
async fn a_real_keyring_answers_from_inside_the_runtime_and_from_a_blocking_thread() {
    let on_async_thread = vault_of_a_test_office().load();
    if let Err(error) = &on_async_thread {
        eprintln!("skipped: no usable OS keyring here ({error})");
        return;
    }

    let on_blocking_thread =
        tokio::task::spawn_blocking(|| vault_of_a_test_office().load().map(|key| key.is_some()))
            .await
            .expect("blocking task");

    assert!(on_blocking_thread.is_ok(), "{on_blocking_thread:?}");
}

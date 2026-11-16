// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use crate::common::errors::AccessError;
use crate::common::interfaces::TokenStore;

/// Replaces the office token. The store announces the new generation, and the
/// server closes every connection that used an older one (FR-010).
#[derive(Clone)]
pub struct RegenerateTokenHandler {
    store: Arc<dyn TokenStore>,
}

impl RegenerateTokenHandler {
    #[must_use]
    pub fn new(store: Arc<dyn TokenStore>) -> Self {
        Self { store }
    }

    /// Returns the new generation, never the token.
    ///
    /// # Errors
    /// The store's error; the old token stays in force.
    pub fn handle(&self) -> Result<u64, AccessError> {
        Ok(self.store.regenerate()?.generation)
    }
}

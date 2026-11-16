// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use access_application::commands::authenticate_connection::AuthenticationHandler;
use access_application::commands::regenerate_token::RegenerateTokenHandler;
use access_application::common::errors::AccessError;
use access_infrastructure::services::{FileTokenStore, HmacProofScheme, RandomNonceSource};
use access_presentation::AccessHandlers;

use crate::config::Config;

/// The access module, wired: the token file, the authentication use case that
/// the server's auth gate calls, and the `token.*` handlers.
pub struct AccessServices {
    pub token_store: Arc<FileTokenStore>,
    pub authentication: AuthenticationHandler,
    pub handlers: AccessHandlers,
}

/// Opens the token file, creating it with a new token on the first start.
///
/// # Errors
/// The token file cannot be read, written, or parsed.
pub fn build_access_services(config: &Config) -> Result<AccessServices, AccessError> {
    let token_store = Arc::new(FileTokenStore::open(config.token_path())?);
    let authentication = AuthenticationHandler::new(
        token_store.clone(),
        Arc::new(RandomNonceSource),
        Arc::new(HmacProofScheme),
    );
    let handlers = AccessHandlers::new(RegenerateTokenHandler::new(token_store.clone()));
    Ok(AccessServices {
        token_store,
        authentication,
        handlers,
    })
}

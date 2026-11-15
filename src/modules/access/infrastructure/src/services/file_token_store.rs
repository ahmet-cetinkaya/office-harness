// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs::OpenOptions;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use access_application::common::errors::AccessError;
use access_application::common::interfaces::TokenStore;
use access_application::common::models::TokenSnapshot;
use access_domain::value_objects::OfficeToken;
use private_fs::{open_private_file, replace_private_file};
use tokio::sync::watch;

use super::random_bytes::random_bytes;

const FIRST_GENERATION: u64 = 1;

/// The office token in the owner-only `token` file. It announces each new
/// generation on a watch channel, which the server uses to close the
/// connections of the old token.
pub struct FileTokenStore {
    path: PathBuf,
    state: Mutex<TokenSnapshot>,
    generations: watch::Sender<u64>,
}

impl FileTokenStore {
    /// Reads the token file, or creates it with a new token when it is
    /// missing or empty.
    ///
    /// # Errors
    /// `Storage` when the file cannot be read or written, `InvalidToken` when
    /// it holds something that is not a token (it is never replaced silently),
    /// and `Randomness` when a new token cannot be made.
    pub fn open(path: PathBuf) -> Result<Self, AccessError> {
        let token = match read_token_text(&path)? {
            Some(text) => OfficeToken::parse(&text)?,
            None => create_token(&path)?,
        };
        let (generations, _) = watch::channel(FIRST_GENERATION);
        Ok(Self {
            path,
            state: Mutex::new(TokenSnapshot {
                token,
                generation: FIRST_GENERATION,
            }),
            generations,
        })
    }

    /// The latest generation, now and at each regeneration.
    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.generations.subscribe()
    }

    fn lock_state(&self) -> MutexGuard<'_, TokenSnapshot> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl TokenStore for FileTokenStore {
    fn current(&self) -> Result<TokenSnapshot, AccessError> {
        Ok(self.lock_state().clone())
    }

    fn regenerate(&self) -> Result<TokenSnapshot, AccessError> {
        let mut state = self.lock_state();
        let token = create_token(&self.path)?;
        state.token = token;
        state.generation += 1;
        self.generations.send_replace(state.generation);
        Ok(state.clone())
    }
}

/// The file's text, or `None` when it is missing or empty. Opening it also
/// narrows a file that exists with wider access.
fn read_token_text(path: &Path) -> Result<Option<String>, AccessError> {
    let mut file = open_private_file(path, OpenOptions::new().read(true).write(true))
        .map_err(|error| storage(&error))?;
    let mut text = String::new();
    file.read_to_string(&mut text)
        .map_err(|error| storage(&error))?;
    Ok((!text.trim().is_empty()).then_some(text))
}

fn create_token(path: &Path) -> Result<OfficeToken, AccessError> {
    let token = OfficeToken::from_random_bytes(random_bytes()?);
    replace_private_file(path, token.expose_secret().as_bytes())
        .map_err(|error| storage(&error))?;
    Ok(token)
}

fn storage(error: &io::Error) -> AccessError {
    AccessError::Storage(error.to_string())
}

#[cfg(test)]
#[path = "file_token_store.tests.rs"]
mod tests;

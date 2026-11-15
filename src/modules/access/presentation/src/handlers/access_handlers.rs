// SPDX-License-Identifier: AGPL-3.0-or-later

use access_application::commands::regenerate_token::RegenerateTokenHandler;
use serde_json::Value;
use wire::{ErrorCode, Request, Response};

/// The host sends every request whose type starts with this to the module.
pub const TOKEN_PREFIX: &str = "token.";

const REGENERATE: &str = "token.regenerate";

/// Answers the `token.*` requests. The new token is never in the answer; the
/// desktop core reads it from the token file.
#[derive(Clone)]
pub struct AccessHandlers {
    regenerate_token: RegenerateTokenHandler,
}

impl AccessHandlers {
    #[must_use]
    pub fn new(regenerate_token: RegenerateTokenHandler) -> Self {
        Self { regenerate_token }
    }

    #[must_use]
    pub fn handle(&self, request: &Request) -> Response {
        match request.kind.as_str() {
            REGENERATE => self.regenerate(request.id),
            other => Response::failure(
                request.id,
                ErrorCode::NotFound,
                format!("no request of the type {other}"),
            ),
        }
    }

    fn regenerate(&self, id: u32) -> Response {
        match self.regenerate_token.handle() {
            Ok(_) => Response::success(id, Value::Null),
            Err(error) => Response::failure(id, ErrorCode::Internal, error.to_string()),
        }
    }
}

#[cfg(test)]
#[path = "access_handlers.tests.rs"]
mod tests;

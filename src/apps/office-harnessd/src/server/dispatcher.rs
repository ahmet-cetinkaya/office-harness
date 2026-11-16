// SPDX-License-Identifier: AGPL-3.0-or-later

use access_presentation::{AccessHandlers, TOKEN_PREFIX};
use wire::{ErrorCode, Request, Response};

/// Sends each request to the module that owns the prefix of its type. Only
/// the access module answers in this change; the others register here.
#[derive(Clone)]
pub struct Dispatcher {
    access: AccessHandlers,
}

impl Dispatcher {
    #[must_use]
    pub fn new(access: AccessHandlers) -> Self {
        Self { access }
    }

    #[must_use]
    pub fn dispatch(&self, request: &Request) -> Response {
        if request.kind.starts_with(TOKEN_PREFIX) {
            return self.access.handle(request);
        }
        Response::failure(
            request.id,
            ErrorCode::NotFound,
            format!("no request of the type {}", request.kind),
        )
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later

mod handler_error;
mod template_handlers;

pub use handler_error::HandlerError;
pub use template_handlers::{Handled, REQUEST_PREFIX, TemplateHandlers};

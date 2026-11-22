// SPDX-License-Identifier: AGPL-3.0-or-later

pub mod migrations;
mod template_repository;

pub use migrations::{LEDGER_TABLE, SCRIPTS};
pub use template_repository::{SharedHandle, TemplateRepository};

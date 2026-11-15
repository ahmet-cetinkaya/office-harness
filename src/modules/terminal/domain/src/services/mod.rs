// SPDX-License-Identifier: AGPL-3.0-or-later

mod batch_script_rules;
mod query_scanner;

pub use batch_script_rules::{
    first_unsafe_batch_argument, first_unsafe_batch_character, is_batch_script,
};
pub use query_scanner::{FoundQuery, QueryScanner};

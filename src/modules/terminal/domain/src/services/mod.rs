// SPDX-License-Identifier: AGPL-3.0-or-later

mod batch_script_rules;
mod input_draft;
mod input_line;
mod open_question;
mod query_scanner;

pub use batch_script_rules::{
    first_unsafe_batch_argument, first_unsafe_batch_character, is_batch_script,
};
pub use input_draft::InputDraft;
pub use input_line::shows_typed_text;
pub use open_question::shows_open_question;
pub use query_scanner::{FoundQuery, QueryScanner};

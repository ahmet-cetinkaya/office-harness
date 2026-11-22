// SPDX-License-Identifier: AGPL-3.0-or-later

mod file_rules;
mod status_machine;

pub use file_rules::{
    BINARY_PREFIX_BYTES, COMMITS_PER_PAGE, LIST_BUDGET_BYTES, MAX_CHANGED_FILES, MAX_CONTENT_BYTES,
    MAX_LIST_ENTRIES, MAX_MESSAGE_BYTES, MAX_SKIP, MAX_STATUS_OUTPUT_BYTES, MAX_SUBJECT_CHARS,
    RelativePath, is_binary, is_secret_looking,
};
pub use status_machine::next_status;

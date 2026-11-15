// SPDX-License-Identifier: AGPL-3.0-or-later

//! Audit module domain: the action record entries. No I/O.

mod action_error;
mod action_kind;
mod action_record;
mod action_result;

pub use action_error::ActionError;
pub use action_kind::ActionKind;
pub use action_record::{ActionEntry, ActionRecord};
pub use action_result::ActionResult;

#[cfg(test)]
#[path = "action_record.tests.rs"]
mod tests;

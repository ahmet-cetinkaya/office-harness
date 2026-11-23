// SPDX-License-Identifier: AGPL-3.0-or-later

//! Secret module presentation (0026): the wire form of the secret requests.
//! It reads the fields, calls the service, and says what the host must
//! record and publish. No answer, event, or journal text holds a value, except
//! the answer of an agent's `secret.get`.

mod agent_requests;
mod errors;
mod outcome;
mod user_requests;

pub use agent_requests::{answer_available, answer_get, not_a_field};
pub use outcome::{ChangeEvent, Handled, JournalEntry, secret_changed_event};
pub use user_requests::{USER_REQUESTS, answer_user_request};

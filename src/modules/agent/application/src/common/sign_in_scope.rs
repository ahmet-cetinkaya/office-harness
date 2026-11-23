// SPDX-License-Identifier: AGPL-3.0-or-later

//! The answer of a sign-in check that ran before a start took its turn in
//! the status order, kept for that start on the same thread (0019/FR-009,
//! D13), the way [`crate::common::actor_scope::ActorScope`] keeps the actor.

use std::cell::RefCell;

use kernel::AgentId;

use crate::common::models::StatusRun;

/// What a sign-in check decided for one start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignInDecision {
    /// The CLI is not signed in; the text is the reason (FR-007).
    Refused(String),
    /// The start goes on, with the note when the check gave no answer.
    GoOn(Option<String>),
}

struct Checked {
    agent_id: AgentId,
    run: StatusRun,
    decision: SignInDecision,
}

thread_local! {
    static CHECKED: RefCell<Option<Checked>> = const { RefCell::new(None) };
}

/// Keeps one check's answer until it drops at the end of the request, so
/// a later request on the same pooled thread never finds it.
#[must_use = "the answer is gone when the scope drops"]
pub struct SignInScope {
    _private: (),
}

impl SignInScope {
    pub(crate) fn enter(agent_id: AgentId, run: StatusRun, decision: SignInDecision) -> Self {
        CHECKED.with(|checked| {
            *checked.borrow_mut() = Some(Checked {
                agent_id,
                run,
                decision,
            });
        });
        Self { _private: () }
    }

    /// The run that was checked for `agent_id` and its decision, taken once.
    pub(crate) fn take(agent_id: AgentId) -> Option<(StatusRun, SignInDecision)> {
        CHECKED.with(|checked| {
            let mut checked = checked.borrow_mut();
            if checked.as_ref()?.agent_id != agent_id {
                return None;
            }
            checked.take().map(|kept| (kept.run, kept.decision))
        })
    }
}

impl Drop for SignInScope {
    fn drop(&mut self) {
        CHECKED.with(|checked| *checked.borrow_mut() = None);
    }
}

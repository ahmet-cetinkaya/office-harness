// SPDX-License-Identifier: AGPL-3.0-or-later

use std::cell::{Cell, RefCell};
use std::sync::Arc;

use kernel::Actor;

thread_local! {
    static CURRENT: Cell<Actor> = const { Cell::new(Actor::User) };
    static LABEL: RefCell<Option<Arc<str>>> = const { RefCell::new(None) };
}

/// Restores the actor and the label that were current before
/// [`ActorScope::enter`] or [`ActorScope::enter_labeled`].
///
/// Every use case shares one journal and a request runs on one blocking
/// thread, so one scope guard replaces an actor parameter on every command.
#[must_use]
pub struct ActorScope {
    previous_actor: Actor,
    previous_label: Option<Arc<str>>,
}

impl ActorScope {
    /// Enters `actor` with no label.
    pub fn enter(actor: Actor) -> Self {
        Self::enter_with(actor, None)
    }

    /// Enters `actor` with a label that the journal writes beside it, such as
    /// the name a schedule has now (0025 D7).
    pub fn enter_labeled(actor: Actor, label: impl Into<Arc<str>>) -> Self {
        Self::enter_with(actor, Some(label.into()))
    }

    fn enter_with(actor: Actor, label: Option<Arc<str>>) -> Self {
        let previous_actor = CURRENT.replace(actor);
        let previous_label = LABEL.replace(label);
        Self {
            previous_actor,
            previous_label,
        }
    }

    #[must_use]
    pub fn label() -> Option<Arc<str>> {
        LABEL.with_borrow(Clone::clone)
    }

    #[must_use]
    pub fn current() -> Actor {
        CURRENT.get()
    }
}

impl Drop for ActorScope {
    fn drop(&mut self) {
        CURRENT.set(self.previous_actor);
        LABEL.set(self.previous_label.take());
    }
}

#[cfg(test)]
#[path = "actor_scope.tests.rs"]
mod tests;

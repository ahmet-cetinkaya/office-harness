// SPDX-License-Identifier: AGPL-3.0-or-later

use std::cell::Cell;

use kernel::Actor;

thread_local! {
    static CURRENT: Cell<Actor> = const { Cell::new(Actor::User) };
}

/// Restores the actor that was current before [`ActorScope::enter`].
///
/// Every use case shares one journal and a request runs on one blocking
/// thread, so one scope guard replaces an actor parameter on every command.
#[must_use]
pub struct ActorScope(Actor);

impl ActorScope {
    pub fn enter(actor: Actor) -> Self {
        let previous = Self::current();
        CURRENT.set(actor);
        Self(previous)
    }

    #[must_use]
    pub fn current() -> Actor {
        CURRENT.get()
    }
}

impl Drop for ActorScope {
    fn drop(&mut self) {
        CURRENT.set(self.0);
    }
}

#[cfg(test)]
#[path = "actor_scope.tests.rs"]
mod tests;

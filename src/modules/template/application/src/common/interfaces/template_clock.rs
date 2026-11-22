// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::Timestamp;

/// The wall clock, as the agent module's `Clock`.
pub trait TemplateClock: Send + Sync {
    fn now(&self) -> Timestamp;
}

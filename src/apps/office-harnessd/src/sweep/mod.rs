// SPDX-License-Identifier: AGPL-3.0-or-later

//! The orphan sweep of 0017: after an unclean stop, the processes that the
//! office stored for the `lost` sessions are ended when they still are those
//! processes, and every result is recorded. The gate holds the user's starts
//! until the sweep is over.

mod identity_sweep;
mod leftover_ledger;
mod leftover_result;
mod leftover_sweep;
mod orphan_sweep;
mod sweep_gate;
mod visible_form;

pub use leftover_ledger::SharedLedger;
pub use leftover_result::{LeftReason, LeftoverResult};
pub use leftover_sweep::{Leftover, LeftoverNotifier, LeftoverSweep, SweepLedger};
pub use orphan_sweep::OrphanSweep;
pub use sweep_gate::{ServerStopping, SweepGate, SweepState, sweep_event};
pub use visible_form::{escape_unsafe, visible_command};

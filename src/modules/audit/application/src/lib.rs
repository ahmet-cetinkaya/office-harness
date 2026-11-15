// SPDX-License-Identifier: AGPL-3.0-or-later

//! Audit module use cases: write and read the action record. The host app
//! implements each writing module's `ActionRecorder` port with
//! `RecordActionHandler`.

pub mod commands;
pub mod common;
pub mod queries;

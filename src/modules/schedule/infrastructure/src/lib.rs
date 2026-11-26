// SPDX-License-Identifier: AGPL-3.0-or-later

//! Schedule module infrastructure: the SQLite store of the schedules and
//! their run history, and the `schedule_` migrations. The host opens the
//! database file owner-only and hands the connection over.

pub mod persistence;

// SPDX-License-Identifier: AGPL-3.0-or-later

//! Terminal module domain: sizes, output limits, the terminal queries the
//! server answers itself, and the events a terminal emits. No I/O.

pub mod errors;
pub mod events;
pub mod services;
pub mod value_objects;

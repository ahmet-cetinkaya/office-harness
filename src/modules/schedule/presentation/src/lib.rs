// SPDX-License-Identifier: AGPL-3.0-or-later

//! Schedule module protocol surface: the `schedule.*` requests, their JSON
//! forms, and the `schedule.changed` and `schedule.removed` events (0025 plan
//! D16). The host answers `schedule.run_now` through the `RunNow` port.

pub mod contracts;
pub mod handlers;
pub mod mappings;

pub use contracts::refuse_unknown_fields;
pub use handlers::{Handled, HandlerError, REQUEST_PREFIX, ScheduleHandlers};
pub use mappings::{
    field_errors_json, run_json, schedule_changed_event, schedule_json, schedule_removed_event,
};

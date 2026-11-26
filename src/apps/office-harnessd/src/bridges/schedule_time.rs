// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt::Debug;

use agent_application::common::interfaces::Clock;
use jiff::tz::TimeZone;
use kernel::Timestamp;
use schedule_application::common::interfaces::ScheduleClock;
use schedule_domain::ZoneSource;

use super::SystemClock;

/// The wall clock and the time zone that schedules read together. The server
/// uses the machine's; a test passes a fake that it controls.
pub trait ScheduleTime: Send + Sync + Debug {
    fn now(&self) -> Timestamp;
    fn zone(&self) -> TimeZone;
}

/// The machine's clock and its current time zone, read at each call so a zone
/// change shows within one scheduler tick (0025/FR-004).
#[derive(Debug, Clone, Copy)]
pub struct SystemScheduleTime;

impl ScheduleTime for SystemScheduleTime {
    fn now(&self) -> Timestamp {
        SystemClock.now()
    }

    fn zone(&self) -> TimeZone {
        TimeZone::system()
    }
}

/// Gives the schedule application its clock and its zone from one source.
#[derive(Clone, Copy)]
pub struct SharedScheduleTime(pub &'static dyn ScheduleTime);

impl ScheduleClock for SharedScheduleTime {
    fn now(&self) -> Timestamp {
        self.0.now()
    }
}

impl ZoneSource for SharedScheduleTime {
    fn current(&self) -> TimeZone {
        self.0.zone()
    }
}

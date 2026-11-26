// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use kernel::{AgentId, ScheduleId};
use schedule_domain::{
    FieldError, NAME_FIELD, Repeat, Rule, Schedule, TARGET_FIELD, ZoneSource, normalize_name,
    validate_fields,
};

use crate::common::errors::{ScheduleApplicationError, SwitchRefusal};
use crate::common::interfaces::{ScheduleCapSource, ScheduleClock, ScheduleStore, TargetCheck};
use crate::common::models::ScheduleRun;

/// The fields the person saves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveInput {
    /// `None` makes a new schedule; an id edits that one.
    pub id: Option<ScheduleId>,
    pub name: String,
    pub text: String,
    pub target: AgentId,
    pub repeat: Repeat,
    pub switched_on: bool,
}

/// Saves, switches, deletes, and lists schedules. A save keeps nothing when
/// any field breaks a rule, and a schedule is switched off, never run, by it.
#[derive(Clone)]
pub struct ScheduleService {
    store: Arc<dyn ScheduleStore>,
    clock: Arc<dyn ScheduleClock>,
    cap: Arc<dyn ScheduleCapSource>,
    targets: Arc<dyn TargetCheck>,
    zone: Arc<dyn ZoneSource + Send + Sync>,
}

impl ScheduleService {
    #[must_use]
    pub fn new(
        store: Arc<dyn ScheduleStore>,
        clock: Arc<dyn ScheduleClock>,
        cap: Arc<dyn ScheduleCapSource>,
        targets: Arc<dyn TargetCheck>,
        zone: Arc<dyn ZoneSource + Send + Sync>,
    ) -> Self {
        Self {
            store,
            clock,
            cap,
            targets,
            zone,
        }
    }

    /// # Errors
    /// `Storage`.
    pub fn list(&self) -> Result<Vec<Schedule>, ScheduleApplicationError> {
        self.store.list()
    }

    /// The identifier of the machine's time zone now, for the view to format
    /// due times in; `None` for a zone that has none.
    #[must_use]
    pub fn zone_name(&self) -> Option<String> {
        self.zone.current().iana_name().map(str::to_owned)
    }

    /// # Errors
    /// `NotFound`, `Storage`.
    pub fn get(&self, id: ScheduleId) -> Result<Schedule, ScheduleApplicationError> {
        self.store.get(id)?.ok_or_else(|| not_found(id))
    }

    /// Creates a schedule, or replaces the one with `id`. A save clears the
    /// off reason, marks every due time up to now as handled (so the time the
    /// schedule was off, or before it was made, never runs), and computes the
    /// next due time. The anchor of an every-N-hours repeat moves only when
    /// the person saves a changed repeat or a schedule that was off now on.
    ///
    /// # Errors
    /// `Invalid` with every failing field, `CapReached`, `NotFound` for an
    /// unknown `id`, `Storage`.
    pub fn save(&self, mut input: SaveInput) -> Result<Schedule, ScheduleApplicationError> {
        input.name = normalize_name(&input.name);
        let now = self.clock.now();
        let zone = self.zone.current();
        let existing = input.id.map(|id| self.get(id)).transpose()?;

        let mut errors = validate_fields(
            &input.name,
            &input.text,
            &input.repeat,
            input.switched_on,
            now,
            &zone,
        );
        if !self.targets.exists(input.target) {
            errors.push(FieldError {
                field: TARGET_FIELD,
                rule: Rule::TargetMissing,
            });
        }
        if !errors.is_empty() {
            return Err(ScheduleApplicationError::Invalid(errors));
        }

        let mut schedule = match existing {
            Some(existing) => edited(&existing, input, now),
            None => created(input, now),
        };
        schedule.mark_handled_through(now);
        schedule.recompute_next_due(now, &zone);
        self.store
            .save(&schedule, self.cap.cap())
            .map_err(name_rule)?;
        Ok(schedule)
    }

    /// Switches a schedule on or off. Off clears the next due time. On is
    /// refused for a once schedule whose time has passed and for a schedule
    /// with an off reason, until the person edits it; a switch on moves the
    /// anchor and makes no run for the time it was off.
    ///
    /// # Errors
    /// `NotFound`, `SwitchRefused`, `Storage`.
    pub fn switch(&self, id: ScheduleId, on: bool) -> Result<Schedule, ScheduleApplicationError> {
        let mut schedule = self.get(id)?;
        if schedule.switched_on == on {
            return Ok(schedule);
        }
        let now = self.clock.now();
        let zone = self.zone.current();
        if on {
            if let Some(reason) = schedule.off_reason {
                return Err(ScheduleApplicationError::SwitchRefused(reason.into()));
            }
            schedule.anchor = now;
            schedule.mark_handled_through(now);
            if matches!(schedule.repeat, Repeat::Once { .. })
                && schedule.next_due(now, &zone).is_none()
            {
                return Err(ScheduleApplicationError::SwitchRefused(
                    SwitchRefusal::OnceTimePassed,
                ));
            }
        }
        schedule.switched_on = on;
        schedule.recompute_next_due(now, &zone);
        self.store.save(&schedule, self.cap.cap())?;
        Ok(schedule)
    }

    /// The last 20 runs of the schedule, newest first.
    ///
    /// # Errors
    /// `NotFound`, `Storage`.
    pub fn history(&self, id: ScheduleId) -> Result<Vec<ScheduleRun>, ScheduleApplicationError> {
        self.get(id)?;
        self.store.history(id)
    }

    /// Removes the schedule and its history. The action record, a waiting
    /// directive, and a run in progress are not touched.
    ///
    /// # Errors
    /// `NotFound`, `Storage`.
    pub fn delete(&self, id: ScheduleId) -> Result<(), ScheduleApplicationError> {
        if self.store.delete(id)? {
            Ok(())
        } else {
            Err(not_found(id))
        }
    }
}

fn created(input: SaveInput, now: kernel::Timestamp) -> Schedule {
    Schedule {
        id: ScheduleId::generate(),
        name: input.name,
        text: input.text,
        target: input.target,
        repeat: input.repeat,
        switched_on: input.switched_on,
        anchor: now,
        next_due: None,
        last_handled_due: None,
        last_missed: None,
        off_reason: None,
        notified_since_success: false,
    }
}

fn edited(existing: &Schedule, input: SaveInput, now: kernel::Timestamp) -> Schedule {
    let anchor_moves =
        existing.repeat != input.repeat || (input.switched_on && !existing.switched_on);
    Schedule {
        name: input.name,
        text: input.text,
        target: input.target,
        repeat: input.repeat,
        switched_on: input.switched_on,
        anchor: if anchor_moves { now } else { existing.anchor },
        off_reason: None,
        ..existing.clone()
    }
}

/// The store names a taken name as a plain error; the person sees it as the
/// rule of the `name` field.
fn name_rule(error: ScheduleApplicationError) -> ScheduleApplicationError {
    match error {
        ScheduleApplicationError::NameTaken { .. } => {
            ScheduleApplicationError::Invalid(vec![FieldError {
                field: NAME_FIELD,
                rule: Rule::NameTaken,
            }])
        }
        other => other,
    }
}

fn not_found(id: ScheduleId) -> ScheduleApplicationError {
    ScheduleApplicationError::NotFound(format!("schedule {id} does not exist"))
}

#[cfg(test)]
#[path = "schedule_service.tests.rs"]
mod tests;

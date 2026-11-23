// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use kernel::{AgentId, ScheduleId};
use schedule_application::common::interfaces::RunNow;
use schedule_application::services::{SaveInput, ScheduleService};
use schedule_domain::{FieldError, OffReason, Rule, TARGET_FIELD};
use serde_json::{Value, json};
use wire::{Event, Request, Response};

use super::handler_error::HandlerError;
use crate::contracts::{
    DELETE, HISTORY, LIST, RUN_NOW, ReadError, SAVE, SWITCH, SaveFields, ScheduleIdFields,
    SwitchFields, read_repeat,
};
use crate::mappings::{run_json, schedule_changed_event, schedule_json, schedule_removed_event};

/// The request types of this module all start with this prefix.
pub const REQUEST_PREFIX: &str = "schedule.";

/// The answer to one request, the events that every client gets because of
/// it, the `fields` array of an `invalid` answer, and the disk failure text
/// when the disk is at fault. The wire answer has no slot for `fields` yet,
/// so the host adds it to the error object when it writes the frame.
#[derive(Debug)]
pub struct Handled {
    pub response: Response,
    pub events: Vec<Event>,
    pub error_fields: Option<Value>,
    pub disk_failure: Option<String>,
}

struct Success {
    result: Value,
    events: Vec<Event>,
}

impl Success {
    fn read(result: Value) -> Self {
        Self {
            result,
            events: Vec::new(),
        }
    }

    fn with_event(result: Value, event: Event) -> Self {
        Self {
            result,
            events: vec![event],
        }
    }
}

/// The handlers of the `schedule.*` requests. They block, so the host runs
/// them on a blocking thread.
pub struct ScheduleHandlers {
    service: ScheduleService,
    run_now: Arc<dyn RunNow>,
}

impl ScheduleHandlers {
    #[must_use]
    pub fn new(service: ScheduleService, run_now: Arc<dyn RunNow>) -> Self {
        Self { service, run_now }
    }

    #[must_use]
    pub fn handle(&self, request: &Request) -> Handled {
        let id = request.id;
        match self.route(request) {
            Ok(success) => Handled {
                response: Response::success(id, success.result),
                events: success.events,
                error_fields: None,
                disk_failure: None,
            },
            Err(error) => Handled {
                response: Response::failure(id, error.code, error.message),
                events: Vec::new(),
                error_fields: error.fields,
                disk_failure: error.disk_failure,
            },
        }
    }

    fn route(&self, request: &Request) -> Result<Success, HandlerError> {
        match request.kind.as_str() {
            LIST => self.list(),
            SAVE => self.save(request),
            SWITCH => self.switch(request),
            RUN_NOW => self.run_now(request),
            DELETE => self.delete(request),
            HISTORY => self.history(request),
            other => Err(HandlerError::not_found(format!(
                "no request of the type {other}"
            ))),
        }
    }

    fn list(&self) -> Result<Success, HandlerError> {
        let schedules = self.service.list()?;
        Ok(Success::read(json!({
            "schedules": schedules.iter().map(schedule_json).collect::<Vec<_>>(),
            "zone": self.service.zone_name(),
        })))
    }

    fn save(&self, request: &Request) -> Result<Success, HandlerError> {
        let fields = SaveFields::read(&request.fields).map_err(HandlerError::invalid)?;
        let id = fields
            .schedule_id
            .as_deref()
            .map(parse_schedule_id)
            .transpose()?;
        let (target, repeat) = read_target_and_repeat(&fields)?;
        let schedule = self.service.save(SaveInput {
            id,
            name: fields.name,
            text: fields.text,
            target,
            repeat,
            switched_on: fields.on,
        })?;
        Ok(Success::with_event(
            json!({"schedule": schedule_json(&schedule)}),
            schedule_changed_event(&schedule),
        ))
    }

    fn switch(&self, request: &Request) -> Result<Success, HandlerError> {
        let fields = SwitchFields::read(&request.fields).map_err(HandlerError::invalid)?;
        let schedule = self
            .service
            .switch(parse_schedule_id(&fields.schedule_id)?, fields.on)?;
        Ok(Success::with_event(
            json!({"schedule": schedule_json(&schedule)}),
            schedule_changed_event(&schedule),
        ))
    }

    /// Checks the schedule and hands the run to the host. A schedule whose
    /// target is gone makes no run.
    fn run_now(&self, request: &Request) -> Result<Success, HandlerError> {
        let fields = ScheduleIdFields::read(&request.fields).map_err(HandlerError::invalid)?;
        let schedule = self.service.get(parse_schedule_id(&fields.schedule_id)?)?;
        if schedule.off_reason == Some(OffReason::TargetDeleted) {
            return Err(HandlerError::refused(
                "its target was deleted; edit the schedule to name another target",
            ));
        }
        self.run_now
            .run_now(&schedule)
            .map_err(HandlerError::refused)?;
        Ok(Success::read(
            json!({"schedule_id": schedule.id.to_string()}),
        ))
    }

    fn delete(&self, request: &Request) -> Result<Success, HandlerError> {
        let fields = ScheduleIdFields::read(&request.fields).map_err(HandlerError::invalid)?;
        let id = parse_schedule_id(&fields.schedule_id)?;
        self.service.delete(id)?;
        Ok(Success::with_event(
            json!({"schedule_id": id.to_string()}),
            schedule_removed_event(id),
        ))
    }

    fn history(&self, request: &Request) -> Result<Success, HandlerError> {
        let fields = ScheduleIdFields::read(&request.fields).map_err(HandlerError::invalid)?;
        let id = parse_schedule_id(&fields.schedule_id)?;
        let runs = self.service.history(id)?;
        Ok(Success::read(json!({
            "schedule_id": id.to_string(),
            "runs": runs.iter().map(run_json).collect::<Vec<_>>(),
        })))
    }
}

fn parse_schedule_id(text: &str) -> Result<ScheduleId, HandlerError> {
    text.parse().map_err(|_| {
        HandlerError::invalid(format!("schedule_id is not a valid identifier: {text}"))
    })
}

/// Reads the target and the repeat of a save together, so that one answer
/// names every field that is wrong.
fn read_target_and_repeat(
    fields: &SaveFields,
) -> Result<(AgentId, schedule_domain::Repeat), HandlerError> {
    let target = fields.target_agent_id.parse::<AgentId>();
    let repeat = read_repeat(&fields.repeat);
    match (target, repeat) {
        (Ok(target), Ok(repeat)) => Ok((target, repeat)),
        (_, Err(ReadError::Request(message))) => Err(HandlerError::invalid(message)),
        (target, repeat) => {
            let mut errors = Vec::new();
            if target.is_err() {
                errors.push(FieldError {
                    field: TARGET_FIELD,
                    rule: Rule::TargetMissing,
                });
            }
            if let Err(ReadError::Rule(error)) = repeat {
                errors.push(error);
            }
            Err(HandlerError::invalid_fields(&errors))
        }
    }
}

#[cfg(test)]
#[path = "schedule_handlers.tests.rs"]
mod tests;

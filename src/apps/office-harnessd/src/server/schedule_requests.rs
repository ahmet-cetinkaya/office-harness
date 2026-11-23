// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_application::common::action_journal::ActionJournal;
use agent_application::common::actor_scope::ActorScope;
use agent_application::common::interfaces::AgentStore;
use agent_application::common::models::AgentAction;
use kernel::{Actor, ScheduleId};
use schedule_application::common::interfaces::ScheduleStore;
use schedule_domain::Schedule;
use schedule_presentation::{REQUEST_PREFIX, ScheduleHandlers, refuse_unknown_fields};
use serde_json::Value;
use tokio::task::spawn_blocking;
use wire::{ErrorCode, Request, Response};

use super::event_hub::EventHub;
use crate::bootstrap::StorageAlerts;
use crate::bridges::{ScheduleBridge, take_recorded_events};

const SAVE: &str = "schedule.save";
const SWITCH: &str = "schedule.switch";
const RUN_NOW: &str = "schedule.run_now";
const DELETE: &str = "schedule.delete";

/// Runs the user's `schedule.*` requests. The use cases block, so each call
/// runs on a blocking thread, under the schedule lock so no round or settled
/// run overwrites it. Each save, edit, switch, Run now, and delete of the
/// person goes into the action record of the target while it exists
/// (0025/FR-012). The agent connection never routes here (FR-014).
#[derive(Clone)]
pub struct ScheduleGateway {
    handlers: Arc<ScheduleHandlers>,
    bridge: ScheduleBridge,
    store: Arc<dyn ScheduleStore>,
    agents: Arc<dyn AgentStore>,
    journal: ActionJournal,
    events: EventHub,
    alerts: StorageAlerts,
}

/// What the person did to which schedule, for the action record.
struct PersonAction {
    schedule: Schedule,
    detail: String,
}

impl ScheduleGateway {
    #[must_use]
    pub fn new(
        handlers: Arc<ScheduleHandlers>,
        bridge: ScheduleBridge,
        store: Arc<dyn ScheduleStore>,
        agents: Arc<dyn AgentStore>,
        journal: ActionJournal,
        events: EventHub,
        alerts: StorageAlerts,
    ) -> Self {
        Self {
            handlers,
            bridge,
            store,
            agents,
            journal,
            events,
            alerts,
        }
    }

    /// True for the requests that this gateway answers: the whole of
    /// `schedule.`.
    #[must_use]
    pub fn accepts(kind: &str) -> bool {
        kind.starts_with(REQUEST_PREFIX)
    }

    pub async fn handle(&self, request: Request) -> Response {
        let id = request.id;
        let gateway = self.clone();
        spawn_blocking(move || gateway.run(&request))
            .await
            .unwrap_or_else(|error| {
                Response::failure(
                    id,
                    ErrorCode::Internal,
                    format!("the request task failed: {error}"),
                )
            })
    }

    fn run(&self, request: &Request) -> Response {
        if let Err(message) = refuse_unknown_fields(request) {
            return Response::failure(request.id, ErrorCode::Invalid, message);
        }
        let _locked = self.bridge.lock_schedules();
        // Drop what an earlier request on this thread left behind.
        drop(take_recorded_events());
        let before = self.read_before(request);
        let handled = self.handlers.handle(request);
        if let Some(message) = &handled.disk_failure {
            self.alerts.report_disk_failure(message);
        }
        if let Response::Success { result, .. } = &handled.response {
            self.record_person_action(request, result, before);
        }
        for event in handled.events.iter().chain(&take_recorded_events()) {
            self.events.publish(event);
        }
        handled.response
    }

    /// The schedule that a delete or a Run now names, read before the use
    /// case runs, because a delete takes it away.
    fn read_before(&self, request: &Request) -> Option<Schedule> {
        if request.kind != DELETE && request.kind != RUN_NOW {
            return None;
        }
        let id = schedule_id_of(request.fields.get("schedule_id")?)?;
        self.store.get(id).ok().flatten()
    }

    fn record_person_action(&self, request: &Request, result: &Value, before: Option<Schedule>) {
        let Some(action) = self.person_action(request, result, before) else {
            return;
        };
        // Only while the target exists (FR-012).
        if !matches!(self.agents.find(action.schedule.target), Ok(Some(_))) {
            return;
        }
        let _actor = ActorScope::enter(Actor::User);
        let recorded = self.journal.record_detailed(
            action.schedule.target,
            AgentAction::Directive,
            None,
            Some(&action.detail),
        );
        if let Err(error) = recorded {
            self.alerts
                .report_disk_failure(&format!("the action record failed: {error}"));
        }
    }

    fn person_action(
        &self,
        request: &Request,
        result: &Value,
        before: Option<Schedule>,
    ) -> Option<PersonAction> {
        let (schedule, verb) = match request.kind.as_str() {
            SAVE => {
                let schedule = self.schedule_of(result.get("schedule")?)?;
                let verb = if request.fields.contains_key("schedule_id") {
                    "edited"
                } else {
                    "created"
                };
                (schedule, verb.to_owned())
            }
            SWITCH => {
                let schedule = self.schedule_of(result.get("schedule")?)?;
                let verb = if schedule.switched_on {
                    "switched on"
                } else {
                    "switched off"
                };
                (schedule, verb.to_owned())
            }
            RUN_NOW => (before?, "run now".to_owned()),
            DELETE => (before?, "deleted".to_owned()),
            _ => return None,
        };
        let detail = format!("schedule \"{}\" {verb} by the person", schedule.name);
        Some(PersonAction { schedule, detail })
    }

    fn schedule_of(&self, json: &Value) -> Option<Schedule> {
        let id = schedule_id_of(json.get("id")?)?;
        self.store.get(id).ok().flatten()
    }
}

fn schedule_id_of(value: &Value) -> Option<ScheduleId> {
    value.as_str()?.parse().ok()
}

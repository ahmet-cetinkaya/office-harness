// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex, PoisonError};

use kernel::{AgentId, Timestamp};
use serde_json::{Map, json};
use template_application::common::errors::TemplateApplicationError;
use template_application::common::interfaces::{AgentSource, TemplateClock, TemplateStore};
use template_application::common::models::AgentDefinition;
use template_domain::{Template, TemplateId};
use wire::ErrorCode;

use super::*;
use crate::contracts::refuse_unknown_fields;

#[derive(Default)]
struct MemoryTemplates(Mutex<Vec<Template>>);

impl TemplateStore for MemoryTemplates {
    fn list(&self) -> Result<Vec<Template>, TemplateApplicationError> {
        Ok(self
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone())
    }

    fn find(&self, id: TemplateId) -> Result<Option<Template>, TemplateApplicationError> {
        Ok(self.list()?.into_iter().find(|template| template.id == id))
    }

    fn insert(&self, template: &Template) -> Result<(), TemplateApplicationError> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(template.clone());
        Ok(())
    }

    fn update(&self, _template: &Template) -> Result<(), TemplateApplicationError> {
        unreachable!("these tests never replace a template")
    }

    fn delete(&self, _id: TemplateId) -> Result<bool, TemplateApplicationError> {
        unreachable!("these tests never delete a template")
    }

    fn insert_all(&self, _templates: &[Template]) -> Result<(), TemplateApplicationError> {
        unreachable!("these tests never import")
    }
}

struct NoAgents;

impl AgentSource for NoAgents {
    fn definition_of(&self, id: AgentId) -> Result<AgentDefinition, TemplateApplicationError> {
        Err(TemplateApplicationError::NotFound(format!(
            "agent {id} does not exist"
        )))
    }
}

struct FixedClock;

impl TemplateClock for FixedClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_unix_millis(1)
    }
}

fn handlers() -> TemplateHandlers {
    TemplateHandlers::new(TemplateService::new(
        Arc::new(MemoryTemplates::default()),
        Arc::new(NoAgents),
        Arc::new(FixedClock),
    ))
}

fn request(kind: &str, fields: &serde_json::Value) -> Request {
    let serde_json::Value::Object(fields) = fields else {
        panic!("an object");
    };
    Request {
        id: 7,
        kind: kind.to_owned(),
        fields: Map::from_iter(fields.clone()),
    }
}

fn agent_body() -> serde_json::Value {
    json!({"body_version": 1, "name_pattern": "reviewer-{n}", "command": "claude"})
}

fn failure_of(handled: &Handled) -> (ErrorCode, &str) {
    match &handled.response {
        Response::Failure { code, message, .. } => (*code, message.as_str()),
        other @ Response::Success { .. } => panic!("expected a failure, got {other:?}"),
    }
}

#[test]
fn a_field_the_request_does_not_take_is_named() {
    let misspelt = request(
        "template.create_agent",
        &json!({"templateid": "x", "template_id": "y"}),
    );

    assert_eq!(
        refuse_unknown_fields(&misspelt),
        Err("templateid is not a field of template.create_agent".to_owned())
    );
}

#[test]
fn saving_answers_with_the_template_and_announces_the_full_list() {
    let handlers = handlers();
    let save = request(
        "template.save",
        &json!({"name": "Reviewer", "kind": "agent", "body": agent_body()}),
    );

    let handled = handlers.handle(&save);

    assert!(matches!(handled.response, Response::Success { .. }));
    assert_eq!(handled.events.len(), 1);
    assert!(
        handled.events[0]
            .encode()
            .contains("\"type\":\"template.changed\"")
    );
}

#[test]
fn a_kind_that_does_not_match_the_body_says_so() {
    let save = request(
        "template.save",
        &json!({"name": "Reviewer", "kind": "team", "body": agent_body()}),
    );

    let handled = handlers().handle(&save);

    let (code, message) = failure_of(&handled);
    assert_eq!(code, ErrorCode::Invalid);
    assert_eq!(message, "kind is \"team\" but the body is an agent body");
}

#[test]
fn a_list_changes_nothing_and_sends_no_event() {
    let handled = handlers().handle(&request("template.list", &json!({})));

    assert_eq!(handled.events.len(), 0);
}

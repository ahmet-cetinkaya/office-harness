// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::AgentId;
use serde_json::{Value, json};
use template_application::services::TemplateService;
use template_domain::{TemplateBody, TemplateId, TemplateKind, body_to_json, parse_body};
use wire::{Event, Request, Response};

use super::handler_error::HandlerError;
use crate::contracts::{
    DELETE, EXPORT, ExportFields, FROM_AGENT, FromAgentFields, GET, IMPORT_CONFIRM, IMPORT_PREVIEW,
    ImportFields, LIST, SAVE, SaveFields, TemplateIdFields,
};
use crate::mappings::{preview_json, template_changed_event, template_json};

/// The request types of this module all start with this prefix.
pub const REQUEST_PREFIX: &str = "template.";

/// The answer to one request, the events that every client gets because of
/// it, and the disk failure text when the disk is at fault.
#[derive(Debug)]
pub struct Handled {
    pub response: Response,
    pub events: Vec<Event>,
    pub disk_failure: Option<String>,
}

/// What a successful request answers, and whether it changed the templates.
struct Success {
    result: Value,
    changed_templates: bool,
}

impl Success {
    fn read(result: Value) -> Self {
        Self {
            result,
            changed_templates: false,
        }
    }

    fn changed(result: Value) -> Self {
        Self {
            result,
            changed_templates: true,
        }
    }
}

/// The handlers of the requests that need only this module: the two create
/// requests cross modules, so the host answers them. The handlers block, so
/// the host runs them on a blocking thread.
pub struct TemplateHandlers {
    service: TemplateService,
}

impl TemplateHandlers {
    #[must_use]
    pub fn new(service: TemplateService) -> Self {
        Self { service }
    }

    #[must_use]
    pub fn handle(&self, request: &Request) -> Handled {
        let id = request.id;
        match self.route(request) {
            Ok(success) => self.answered(id, success),
            Err(error) => Handled {
                response: Response::failure(id, error.code, error.message),
                events: Vec::new(),
                disk_failure: error.disk_failure,
            },
        }
    }

    /// The answer, with `template.changed` when the templates changed. A
    /// failed read of the list after the change is a disk fault to report;
    /// the change itself is saved.
    fn answered(&self, id: u32, success: Success) -> Handled {
        let response = Response::success(id, success.result);
        if !success.changed_templates {
            return Handled {
                response,
                events: Vec::new(),
                disk_failure: None,
            };
        }
        match self.service.list() {
            Ok(templates) => Handled {
                response,
                events: vec![template_changed_event(&templates)],
                disk_failure: None,
            },
            Err(error) => Handled {
                response,
                events: Vec::new(),
                disk_failure: error.is_disk_failure().then(|| error.to_string()),
            },
        }
    }

    fn route(&self, request: &Request) -> Result<Success, HandlerError> {
        match request.kind.as_str() {
            LIST => self.list(),
            GET => self.get(request),
            SAVE => self.save(request),
            DELETE => self.delete(request),
            FROM_AGENT => self.copy_agent(request),
            EXPORT => self.export(request),
            IMPORT_PREVIEW => self.preview_import(request),
            IMPORT_CONFIRM => self.confirm_import(request),
            other => Err(HandlerError::not_found(format!(
                "no request of the type {other}"
            ))),
        }
    }

    fn list(&self) -> Result<Success, HandlerError> {
        let templates = self.service.list()?;
        Ok(Success::read(
            json!({"templates": templates.iter().map(template_json).collect::<Vec<_>>()}),
        ))
    }

    fn get(&self, request: &Request) -> Result<Success, HandlerError> {
        let fields = TemplateIdFields::read(&request.fields).map_err(HandlerError::invalid)?;
        let template = self
            .service
            .get(parse_template_id("template_id", &fields.template_id)?)?;
        Ok(Success::read(json!({"template": template_json(&template)})))
    }

    fn save(&self, request: &Request) -> Result<Success, HandlerError> {
        let fields = SaveFields::read(&request.fields).map_err(HandlerError::invalid)?;
        let id = fields
            .template_id
            .as_deref()
            .map(|text| parse_template_id("template_id", text))
            .transpose()?;
        let kind = TemplateKind::parse(&fields.kind)?;
        let body = read_body(kind, &fields.body)?;
        let template = self.service.save(id, &fields.name, body)?;
        Ok(Success::changed(
            json!({"template": template_json(&template)}),
        ))
    }

    fn delete(&self, request: &Request) -> Result<Success, HandlerError> {
        let fields = TemplateIdFields::read(&request.fields).map_err(HandlerError::invalid)?;
        self.service
            .delete(parse_template_id("template_id", &fields.template_id)?)?;
        Ok(Success::changed(json!({})))
    }

    fn copy_agent(&self, request: &Request) -> Result<Success, HandlerError> {
        let fields = FromAgentFields::read(&request.fields).map_err(HandlerError::invalid)?;
        let agent_id: AgentId = fields.agent_id.parse().map_err(|_| {
            HandlerError::invalid(format!(
                "agent_id is not a valid identifier: {}",
                fields.agent_id
            ))
        })?;
        let body = self.service.body_from_agent(agent_id)?;
        Ok(Success::read(
            json!({"body": body_to_json(&TemplateBody::Agent(body))}),
        ))
    }

    fn export(&self, request: &Request) -> Result<Success, HandlerError> {
        let fields = ExportFields::read(&request.fields).map_err(HandlerError::invalid)?;
        let ids = fields
            .ids
            .map(|ids| {
                ids.iter()
                    .map(|text| parse_template_id("ids", text))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?;
        let text = self.service.export(ids.as_deref())?;
        Ok(Success::read(json!({"text": text})))
    }

    fn preview_import(&self, request: &Request) -> Result<Success, HandlerError> {
        let fields = ImportFields::read(&request.fields).map_err(HandlerError::invalid)?;
        let entries = self.service.preview_import(&fields.text)?;
        Ok(Success::read(
            json!({"templates": entries.iter().map(preview_json).collect::<Vec<_>>()}),
        ))
    }

    fn confirm_import(&self, request: &Request) -> Result<Success, HandlerError> {
        let fields = ImportFields::read(&request.fields).map_err(HandlerError::invalid)?;
        let saved = self.service.confirm_import(&fields.text)?;
        Ok(Success::changed(
            json!({"templates": saved.iter().map(template_json).collect::<Vec<_>>()}),
        ))
    }
}

fn parse_template_id(field: &str, text: &str) -> Result<TemplateId, HandlerError> {
    text.parse()
        .map_err(|_| HandlerError::invalid(format!("{field} is not a valid identifier: {text}")))
}

/// Reads the body as the kind says. A body of the other kind gets its own
/// message, which is clearer than the field of the wrong shape that the
/// first read names.
fn read_body(kind: TemplateKind, value: &Value) -> Result<TemplateBody, HandlerError> {
    match parse_body(kind, value) {
        Ok(body) => Ok(body),
        Err(error) => {
            let other = other_kind(kind);
            if parse_body(other, value).is_ok() {
                return Err(HandlerError::invalid(format!(
                    "kind is \"{}\" but the body is {} body",
                    kind.as_str(),
                    with_article(other)
                )));
            }
            Err(error.into())
        }
    }
}

const fn with_article(kind: TemplateKind) -> &'static str {
    match kind {
        TemplateKind::Agent => "an agent",
        TemplateKind::Team => "a team",
    }
}

const fn other_kind(kind: TemplateKind) -> TemplateKind {
    match kind {
        TemplateKind::Agent => TemplateKind::Team,
        TemplateKind::Team => TemplateKind::Agent,
    }
}

#[cfg(test)]
#[path = "template_handlers.tests.rs"]
mod tests;

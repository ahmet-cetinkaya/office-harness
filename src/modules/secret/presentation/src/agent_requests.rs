// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::AgentId;
use secret_application::{SecretService, SecretServiceError};
use secret_domain::SecretName;
use serde_json::{Map, Value, json};
use wire::{ErrorCode, Request, Response};

use crate::errors::code_of;
use crate::outcome::{Handled, JournalEntry};

/// The most characters of a client's field name that a refusal keeps, so one
/// hostile frame cannot flood the record.
const MAX_FIELD_CHARS: usize = 64;

/// `"<field>" is not a field of <request>`, for the first field a request
/// does not take. The field is quoted and cut, so a line break or a long name
/// cannot fake or flood a record line.
#[must_use]
pub fn not_a_field(request: &Request, taken: &[&str]) -> Option<String> {
    request
        .fields
        .keys()
        .find(|field| !taken.contains(&field.as_str()))
        .map(|field| {
            let field: String = field.chars().take(MAX_FIELD_CHARS).collect();
            format!("{field:?} is not a field of {}", request.kind)
        })
}

/// `secret.available`: the names and versions the caller may read.
#[must_use]
pub fn answer_available(service: &SecretService, caller: AgentId, request: &Request) -> Response {
    if let Some(message) = not_a_field(request, &[]) {
        return Response::failure(request.id, ErrorCode::Invalid, message);
    }
    match service.available(&caller) {
        Ok(secrets) => {
            let secrets: Vec<Value> = secrets
                .iter()
                .map(|secret| json!({"name": secret.name.as_str(), "version": secret.version.number()}))
                .collect();
            Response::success(request.id, json!({"secrets": secrets}))
        }
        Err(error) => Response::failure(request.id, code_of(&error), error.to_string()),
    }
}

/// `secret.get`: the current value of a granted secret. The entry for the
/// action record is returned with the answer, for an accepted read and for
/// every refusal alike.
#[must_use]
pub fn answer_get(service: &SecretService, caller: AgentId, request: &Request) -> Handled {
    if let Some(message) = not_a_field(request, &["name"]) {
        return refused(
            request.id,
            caller,
            message.clone(),
            ErrorCode::Invalid,
            message,
        );
    }
    let name = match name_of(&request.fields) {
        Ok(name) => name,
        Err(message) => {
            return refused(
                request.id,
                caller,
                message.clone(),
                ErrorCode::Invalid,
                message,
            );
        }
    };
    match service.read(&caller, &name) {
        Ok(answer) => Handled {
            response: Response::success(
                request.id,
                json!({
                    "name": answer.name.as_str(),
                    "value": answer.value.reveal(),
                    "version": answer.version.number(),
                }),
            ),
            entry: Some(JournalEntry::Read {
                agent: caller,
                text: format!("{} v{}", answer.name, answer.version.number()),
                failed: false,
            }),
            event: Some(crate::outcome::ChangeEvent {
                cause: "read",
                name: Some(answer.name.to_string()),
                agent_id: Some(caller),
            }),
        },
        Err(error) => {
            let record_reason = match &error {
                SecretServiceError::NotAvailable { .. } => "not available".to_owned(),
                other => other.to_string(),
            };
            let text = format!("{name}: {record_reason}");
            refused(request.id, caller, text, code_of(&error), error.to_string())
        }
    }
}

fn name_of(fields: &Map<String, Value>) -> Result<SecretName, String> {
    let text = fields
        .get("name")
        .and_then(Value::as_str)
        .ok_or("name is not a text")?;
    SecretName::parse(text).map_err(|error| error.to_string())
}

/// A refusal: the caller sees `message`; the record keeps `text`.
fn refused(id: u32, caller: AgentId, text: String, code: ErrorCode, message: String) -> Handled {
    Handled {
        response: Response::failure(id, code, message),
        entry: Some(JournalEntry::Read {
            agent: caller,
            text,
            failed: true,
        }),
        event: None,
    }
}

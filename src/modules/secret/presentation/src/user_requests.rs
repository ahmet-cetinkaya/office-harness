// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::AgentId;
use secret_application::{
    GrantSummary, SecretService, SecretServiceError, SecretSummary, StoreState,
};
use secret_domain::{Custody, SecretName, SecretValue};
use serde_json::{Map, Value, json};
use wire::{ErrorCode, Request, Response};

use crate::agent_requests::not_a_field;
use crate::errors::code_of;
use crate::outcome::{ChangeEvent, Handled, JournalEntry};

pub const USER_REQUESTS: [&str; 6] = [
    "secret.list",
    "secret.set",
    "secret.delete",
    "secret.grant",
    "secret.revoke",
    "secret.reset",
];

type Failure = (ErrorCode, String);

/// Answers one user request. It reads the fields it takes and refuses any
/// other (`<field> is not a field of <request>`); the value is moved out of
/// the request so that the request holds no second copy.
pub fn answer_user_request(service: &SecretService, mut request: Request) -> Handled {
    if let Some(message) = not_a_field(&request, fields_of(&request.kind)) {
        return Handled::answer(Response::failure(request.id, ErrorCode::Invalid, message));
    }
    let id = request.id;
    match run(service, &mut request) {
        Ok(handled) => handled,
        Err((code, message)) => Handled::answer(Response::failure(id, code, message)),
    }
}

fn fields_of(kind: &str) -> &'static [&'static str] {
    match kind {
        "secret.set" => &["name", "value"],
        "secret.delete" => &["name"],
        "secret.grant" | "secret.revoke" => &["name", "agent_id"],
        "secret.reset" => &["confirm_count"],
        _ => &[],
    }
}

fn run(service: &SecretService, request: &mut Request) -> Result<Handled, Failure> {
    let id = request.id;
    match request.kind.as_str() {
        "secret.list" => list(service, id),
        "secret.set" => set(service, request),
        "secret.delete" => delete(service, request),
        "secret.grant" => grant(service, request),
        "secret.revoke" => revoke(service, request),
        _ => reset(service, request),
    }
}

fn failure(error: &SecretServiceError) -> Failure {
    (code_of(error), error.to_string())
}

fn list(service: &SecretService, id: u32) -> Result<Handled, Failure> {
    let (state, secrets) = service.list().map_err(|error| failure(&error))?;
    let result = json!({
        "store": store_json(state),
        "secrets": secrets.iter().map(secret_json).collect::<Vec<_>>(),
    });
    Ok(Handled::answer(Response::success(id, result)))
}

fn set(service: &SecretService, request: &mut Request) -> Result<Handled, Failure> {
    let name = name_field(&request.fields)?;
    let value = match request.fields.remove("value") {
        Some(Value::String(text)) => {
            SecretValue::parse(text).map_err(|error| (ErrorCode::Invalid, error.to_string()))?
        }
        _ => return Err((ErrorCode::Invalid, "value is not a text".to_owned())),
    };
    let summary = service
        .save(name.clone(), &value)
        .map_err(|error| failure(&error))?;
    let (cause, text) = if summary.version.number() == 1 {
        ("saved", format!("saved {name} v1"))
    } else {
        (
            "replaced",
            format!("replaced {name} v{}", summary.version.number()),
        )
    };
    Ok(changed(
        request.id,
        json!({"secret": secret_json(&summary)}),
        ChangeEvent {
            cause,
            name: Some(name.to_string()),
            agent_id: None,
        },
        None,
        text,
    ))
}

fn delete(service: &SecretService, request: &Request) -> Result<Handled, Failure> {
    let name = name_field(&request.fields)?;
    service.delete(&name).map_err(|error| failure(&error))?;
    Ok(changed(
        request.id,
        json!({}),
        ChangeEvent {
            cause: "deleted",
            name: Some(name.to_string()),
            agent_id: None,
        },
        None,
        format!("deleted {name}"),
    ))
}

fn grant(service: &SecretService, request: &Request) -> Result<Handled, Failure> {
    let (name, agent) = name_and_agent(&request.fields)?;
    let summary = service
        .grant(&name, &agent)
        .map_err(|error| failure(&error))?;
    Ok(changed(
        request.id,
        json!({"secret": secret_json(&summary)}),
        grant_event("granted", &name, agent),
        Some(agent),
        format!("granted {name}"),
    ))
}

fn revoke(service: &SecretService, request: &Request) -> Result<Handled, Failure> {
    let (name, agent) = name_and_agent(&request.fields)?;
    let summary = service
        .revoke(&name, &agent)
        .map_err(|error| failure(&error))?;
    Ok(changed(
        request.id,
        json!({"secret": secret_json(&summary)}),
        grant_event("revoked", &name, agent),
        Some(agent),
        format!("revoked {name}"),
    ))
}

fn reset(service: &SecretService, request: &Request) -> Result<Handled, Failure> {
    let confirmed = request
        .fields
        .get("confirm_count")
        .and_then(Value::as_u64)
        .and_then(|count| usize::try_from(count).ok())
        .ok_or((
            ErrorCode::Invalid,
            "confirm_count is not a whole number".to_owned(),
        ))?;
    let removed = match service.reset(confirmed) {
        Ok(removed) => removed,
        Err(SecretServiceError::ConfirmCountDiffers { current }) => {
            return Err((
                ErrorCode::Conflict,
                format!("the store holds {current} secrets, not {confirmed}"),
            ));
        }
        Err(error) => return Err(failure(&error)),
    };
    Ok(changed(
        request.id,
        json!({"removed": removed}),
        ChangeEvent {
            cause: "reset",
            name: None,
            agent_id: None,
        },
        None,
        format!("reset {removed} secrets"),
    ))
}

fn changed(
    id: u32,
    result: Value,
    event: ChangeEvent,
    target: Option<AgentId>,
    text: String,
) -> Handled {
    Handled {
        response: Response::success(id, result),
        entry: Some(JournalEntry::Change { target, text }),
        event: Some(event),
    }
}

fn grant_event(cause: &'static str, name: &SecretName, agent: AgentId) -> ChangeEvent {
    ChangeEvent {
        cause,
        name: Some(name.to_string()),
        agent_id: Some(agent),
    }
}

fn name_field(fields: &Map<String, Value>) -> Result<SecretName, Failure> {
    let text = fields
        .get("name")
        .and_then(Value::as_str)
        .ok_or((ErrorCode::Invalid, "name is not a text".to_owned()))?;
    SecretName::parse(text).map_err(|error| (ErrorCode::Invalid, error.to_string()))
}

fn name_and_agent(fields: &Map<String, Value>) -> Result<(SecretName, AgentId), Failure> {
    let name = name_field(fields)?;
    let agent = fields
        .get("agent_id")
        .and_then(Value::as_str)
        .and_then(AgentId::from_uuid_text)
        .ok_or((ErrorCode::Invalid, "agent_id is not an agent id".to_owned()))?;
    Ok((name, agent))
}

fn store_json(state: StoreState) -> Value {
    match state {
        StoreState::Empty => json!({"state": "empty"}),
        StoreState::Ready(custody) => json!({"state": "ready", "custody": custody_text(custody)}),
        StoreState::Locked(custody, reason) => json!({
            "state": "locked",
            "custody": custody_text(custody),
            "locked_reason": reason.to_string(),
        }),
    }
}

fn custody_text(custody: Custody) -> &'static str {
    custody.as_str()
}

fn secret_json(secret: &SecretSummary) -> Value {
    json!({
        "name": secret.name.as_str(),
        "version": secret.version.number(),
        "created_at": secret.created_at.as_unix_millis(),
        "updated_at": secret.updated_at.as_unix_millis(),
        "unreadable": secret.unreadable,
        "grants": secret.grants.iter().map(grant_json).collect::<Vec<_>>(),
    })
}

fn grant_json(grant: &GrantSummary) -> Value {
    json!({
        "agent_id": grant.agent_id.to_string(),
        "last_read_version": grant.last_read_version.map(secret_domain::SecretVersion::number),
        "last_read_at": grant.last_read_at.map(kernel::Timestamp::as_unix_millis),
    })
}

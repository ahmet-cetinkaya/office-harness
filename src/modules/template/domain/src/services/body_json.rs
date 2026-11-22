// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::{Value, json};

use super::json_object::{KnownObject, item_path, text_of};
use crate::entities::{AdditionsBody, AgentBody, EnvVariable, MemberEntry, TeamBody, TemplateBody};
use crate::enums::{TemplateKind, TemplateRole};
use crate::errors::TemplateError;
use crate::value_objects::NamePattern;

const BODY_VERSION: u64 = 1;
const BODY_ROOT: &str = "body";

/// Reads a body from its JSON (D2). The shape is checked here: an unknown
/// field, a missing field, or a field of the wrong type is refused and named.
/// The save rules are `TemplateBody::validate`.
///
/// # Errors
/// `WrongShape`, `Missing`, `UnknownField`, `UnsupportedBodyVersion`, or
/// `UnknownRole`.
pub fn parse_body(kind: TemplateKind, value: &Value) -> Result<TemplateBody, TemplateError> {
    match kind {
        TemplateKind::Agent => {
            let object =
                KnownObject::read(value, BODY_ROOT, "an agent body", AGENT_FIELDS_WITH_VERSION)?;
            check_version(&object)?;
            read_agent(&object).map(TemplateBody::Agent)
        }
        TemplateKind::Team => {
            let object = KnownObject::read(value, BODY_ROOT, "a team body", TEAM_FIELDS)?;
            check_version(&object)?;
            read_team(&object).map(TemplateBody::Team)
        }
    }
}

const AGENT_FIELDS: &[&str] = &[
    "name_pattern",
    "command",
    "working_folder",
    "additions",
    "role",
];
const AGENT_FIELDS_WITH_VERSION: &[&str] = &[
    "body_version",
    "name_pattern",
    "command",
    "working_folder",
    "additions",
    "role",
];
const TEAM_FIELDS: &[&str] = &["body_version", "lead", "members"];
const MEMBER_FIELDS: &[&str] = &["entry", "count"];
const ADDITIONS_FIELDS: &[&str] = &["flags", "env", "startup_prompt"];
const ENV_FIELDS: &[&str] = &["name", "value"];

fn check_version(object: &KnownObject<'_>) -> Result<(), TemplateError> {
    let version = object.required("body_version")?;
    if version.as_u64() == Some(BODY_VERSION) {
        return Ok(());
    }
    Err(TemplateError::UnsupportedBodyVersion {
        found: version.to_string(),
        supported: BODY_VERSION,
    })
}

fn read_agent(object: &KnownObject<'_>) -> Result<AgentBody, TemplateError> {
    let role = object
        .optional_text("role")?
        .map(TemplateRole::parse)
        .transpose()?;
    let additions = match object.optional("additions") {
        Some(value) => read_additions(value, &object.path_of("additions"))?,
        None => AdditionsBody::default(),
    };
    Ok(AgentBody {
        name_pattern: NamePattern::new(object.required_text("name_pattern")?),
        command: object.required_text("command")?.to_owned(),
        working_folder: object.optional_text("working_folder")?.map(str::to_owned),
        additions,
        role,
    })
}

fn read_additions(value: &Value, path: &str) -> Result<AdditionsBody, TemplateError> {
    let object = KnownObject::read(value, path, "an additions object", ADDITIONS_FIELDS)?;
    let flags = object
        .optional_list("flags")?
        .iter()
        .enumerate()
        .map(|(index, flag)| {
            text_of(flag, &item_path(&object.path_of("flags"), index)).map(str::to_owned)
        })
        .collect::<Result<_, _>>()?;
    let env = object
        .optional_list("env")?
        .iter()
        .enumerate()
        .map(|(index, entry)| read_env(entry, &item_path(&object.path_of("env"), index)))
        .collect::<Result<_, _>>()?;
    Ok(AdditionsBody {
        flags,
        env,
        startup_prompt: object.optional_text("startup_prompt")?.map(str::to_owned),
    })
}

fn read_env(value: &Value, path: &str) -> Result<EnvVariable, TemplateError> {
    let object = KnownObject::read(value, path, "an environment entry", ENV_FIELDS)?;
    Ok(EnvVariable {
        name: object.required_text("name")?.to_owned(),
        value: object.optional_text("value")?.map(str::to_owned),
    })
}

fn read_team(object: &KnownObject<'_>) -> Result<TeamBody, TemplateError> {
    let lead = read_entry(object.required("lead")?, &object.path_of("lead"))?;
    let members = object
        .optional_list("members")?
        .iter()
        .enumerate()
        .map(|(index, member)| read_member(member, &item_path(&object.path_of("members"), index)))
        .collect::<Result<_, _>>()?;
    Ok(TeamBody { lead, members })
}

fn read_member(value: &Value, path: &str) -> Result<MemberEntry, TemplateError> {
    let object = KnownObject::read(value, path, "a member entry", MEMBER_FIELDS)?;
    let count_path = object.path_of("count");
    let count = object
        .required("count")?
        .as_u64()
        .and_then(|count| u32::try_from(count).ok())
        .ok_or(TemplateError::WrongShape {
            path: count_path,
            expected: "a whole number",
        })?;
    Ok(MemberEntry {
        entry: read_entry(object.required("entry")?, &object.path_of("entry"))?,
        count,
    })
}

fn read_entry(value: &Value, path: &str) -> Result<AgentBody, TemplateError> {
    let object = KnownObject::read(value, path, "an agent entry", AGENT_FIELDS)?;
    read_agent(&object)
}

/// The JSON of a body, as stored and as sent on the wire (D2).
#[must_use]
pub fn body_to_json(body: &TemplateBody) -> Value {
    let mut json = match body {
        TemplateBody::Agent(agent) => agent_json(agent),
        TemplateBody::Team(team) => json!({
            "lead": agent_json(&team.lead),
            "members": team.members.iter().map(|member| {
                json!({"entry": agent_json(&member.entry), "count": member.count})
            }).collect::<Vec<_>>(),
        }),
    };
    json["body_version"] = json!(BODY_VERSION);
    json
}

fn agent_json(agent: &AgentBody) -> Value {
    json!({
        "name_pattern": agent.name_pattern.as_str(),
        "command": agent.command,
        "working_folder": agent.working_folder,
        "additions": {
            "flags": agent.additions.flags,
            "env": agent.additions.env.iter().map(|variable| {
                json!({"name": variable.name, "value": variable.value})
            }).collect::<Vec<_>>(),
            "startup_prompt": agent.additions.startup_prompt,
        },
        "role": agent.role.map(TemplateRole::as_str),
    })
}

#[cfg(test)]
#[path = "body_json.tests.rs"]
mod tests;

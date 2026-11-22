// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::{Map, Value};
use wire::Request;

use super::Fields;

pub const LIST: &str = "template.list";
pub const GET: &str = "template.get";
pub const SAVE: &str = "template.save";
pub const DELETE: &str = "template.delete";
pub const FROM_AGENT: &str = "template.from_agent";
pub const CREATE_AGENT: &str = "template.create_agent";
pub const CREATE_TEAM: &str = "template.create_team";
pub const EXPORT: &str = "template.export";
pub const IMPORT_PREVIEW: &str = "template.import.preview";
pub const IMPORT_CONFIRM: &str = "template.import.confirm";

const PLACE_FIELD: &str = "place";
const PLACE_FIELDS: [&str; 2] = ["parent_agent_id", "team_id"];

/// The fields each request takes, and no others (template-protocol.md). A
/// template is named by `template_id`, because `id` is the request's own. A
/// type that is not listed is not this module's, and has no fields here.
fn known_fields(kind: &str) -> &'static [&'static str] {
    match kind {
        GET | DELETE => &["template_id"],
        SAVE => &["template_id", "name", "kind", "body"],
        FROM_AGENT => &["agent_id"],
        CREATE_AGENT => &["template_id", PLACE_FIELD, "project_id"],
        CREATE_TEAM => &["template_id", "team_name", "parent_agent_id", "project_id"],
        EXPORT => &["ids"],
        IMPORT_PREVIEW | IMPORT_CONFIRM => &["text"],
        _ => &[],
    }
}

/// Refuses a field that the request does not take, so a misspelt
/// `template_id` never acts on the office.
///
/// # Errors
/// `<field> is not a field of <request>`.
pub fn refuse_unknown_fields(request: &Request) -> Result<(), String> {
    let known = known_fields(&request.kind);
    match request
        .fields
        .keys()
        .find(|field| !known.contains(&field.as_str()))
    {
        Some(field) => Err(format!("{field} is not a field of {}", request.kind)),
        None => Ok(()),
    }
}

/// `template.get` and `template.delete`.
#[derive(Debug)]
pub struct TemplateIdFields {
    pub template_id: String,
}

impl TemplateIdFields {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        Ok(Self {
            template_id: Fields(body).required_text("template_id")?,
        })
    }
}

/// `template.save`.
#[derive(Debug)]
pub struct SaveFields {
    pub template_id: Option<String>,
    pub name: String,
    pub kind: String,
    pub body: Value,
}

impl SaveFields {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        let fields = Fields(body);
        Ok(Self {
            template_id: fields.optional_text("template_id")?,
            name: fields.required_text("name")?,
            kind: fields.required_text("kind")?,
            body: Value::Object(fields.required_object("body")?.clone()),
        })
    }
}

/// `template.from_agent`.
#[derive(Debug)]
pub struct FromAgentFields {
    pub agent_id: String,
}

impl FromAgentFields {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        Ok(Self {
            agent_id: Fields(body).required_text("agent_id")?,
        })
    }
}

/// `template.export`.
#[derive(Debug)]
pub struct ExportFields {
    pub ids: Option<Vec<String>>,
}

impl ExportFields {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        Ok(Self {
            ids: Fields(body).optional_text_list("ids")?,
        })
    }
}

/// `template.import.preview` and `template.import.confirm`.
#[derive(Debug)]
pub struct ImportFields {
    pub text: String,
}

impl ImportFields {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        Ok(Self {
            text: Fields(body).required_text("text")?,
        })
    }
}

/// The `place` of `template.create_agent`: the parent and the team.
#[derive(Debug)]
pub struct PlaceFields {
    pub parent_agent_id: String,
    pub team_id: String,
}

impl PlaceFields {
    fn read(body: &Map<String, Value>) -> Result<Self, String> {
        if let Some(field) = body
            .keys()
            .find(|field| !PLACE_FIELDS.contains(&field.as_str()))
        {
            return Err(format!("{field} is not a field of {PLACE_FIELD}"));
        }
        let fields = Fields(body);
        Ok(Self {
            parent_agent_id: fields
                .required_text("parent_agent_id")
                .map_err(|reason| format!("{PLACE_FIELD}.{reason}"))?,
            team_id: fields
                .required_text("team_id")
                .map_err(|reason| format!("{PLACE_FIELD}.{reason}"))?,
        })
    }
}

/// `template.create_agent`.
#[derive(Debug)]
pub struct CreateAgentFields {
    pub template_id: String,
    pub place: Option<PlaceFields>,
    pub project_id: Option<String>,
}

impl CreateAgentFields {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        let fields = Fields(body);
        Ok(Self {
            template_id: fields.required_text("template_id")?,
            place: fields
                .optional_object(PLACE_FIELD)?
                .map(PlaceFields::read)
                .transpose()?,
            project_id: fields.optional_text("project_id")?,
        })
    }
}

/// `template.create_team`.
#[derive(Debug)]
pub struct CreateTeamFields {
    pub template_id: String,
    pub team_name: String,
    pub parent_agent_id: String,
    pub project_id: Option<String>,
}

impl CreateTeamFields {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        let fields = Fields(body);
        Ok(Self {
            template_id: fields.required_text("template_id")?,
            team_name: fields.required_text("team_name")?,
            parent_agent_id: fields.required_text("parent_agent_id")?,
            project_id: fields.optional_text("project_id")?,
        })
    }
}

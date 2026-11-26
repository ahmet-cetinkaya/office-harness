// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::{Map, Value};
use wire::Request;

use super::Fields;

pub const LIST: &str = "schedule.list";
pub const SAVE: &str = "schedule.save";
pub const SWITCH: &str = "schedule.switch";
pub const RUN_NOW: &str = "schedule.run_now";
pub const DELETE: &str = "schedule.delete";
pub const HISTORY: &str = "schedule.history";

/// The fields each request takes, and no others. A schedule is named by
/// `schedule_id`, because `id` is the request's own.
fn known_fields(kind: &str) -> &'static [&'static str] {
    match kind {
        SAVE => &[
            "schedule_id",
            "name",
            "text",
            "target_agent_id",
            "repeat",
            "on",
        ],
        SWITCH => &["schedule_id", "on"],
        RUN_NOW | DELETE | HISTORY => &["schedule_id"],
        _ => &[],
    }
}

/// Refuses a field that the request does not take, so a misspelt
/// `schedule_id` never acts on the office.
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

/// `schedule.run_now`, `schedule.delete`, and `schedule.history`.
#[derive(Debug)]
pub struct ScheduleIdFields {
    pub schedule_id: String,
}

impl ScheduleIdFields {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        Ok(Self {
            schedule_id: Fields(body).required_text("schedule_id")?,
        })
    }
}

/// `schedule.switch`.
#[derive(Debug)]
pub struct SwitchFields {
    pub schedule_id: String,
    pub on: bool,
}

impl SwitchFields {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        let fields = Fields(body);
        Ok(Self {
            schedule_id: fields.required_text("schedule_id")?,
            on: fields.required_bool("on")?,
        })
    }
}

/// `schedule.save`: a new schedule without `schedule_id`, an edit with one.
#[derive(Debug)]
pub struct SaveFields {
    pub schedule_id: Option<String>,
    pub name: String,
    pub text: String,
    pub target_agent_id: String,
    pub repeat: Map<String, Value>,
    pub on: bool,
}

impl SaveFields {
    /// # Errors
    /// The text names the field at fault.
    pub fn read(body: &Map<String, Value>) -> Result<Self, String> {
        let fields = Fields(body);
        Ok(Self {
            schedule_id: fields.optional_text("schedule_id")?,
            name: fields.required_text("name")?,
            text: fields.required_text("text")?,
            target_agent_id: fields.required_text("target_agent_id")?,
            repeat: fields.required_object("repeat")?.clone(),
            on: fields.required_bool("on")?,
        })
    }
}

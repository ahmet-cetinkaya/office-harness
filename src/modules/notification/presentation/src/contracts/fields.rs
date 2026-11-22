// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::{Map, Value};

/// The body of a request, read field by field so that every failure names the
/// field. (A derived reader says "invalid type" without the name.)
pub struct Fields<'a>(pub &'a Map<String, Value>);

impl Fields<'_> {
    /// # Errors
    /// The body has a field that `request` does not take, so a misspelt
    /// field never acts on the office.
    pub fn refuse_other_than(&self, request: &str, known: &[&str]) -> Result<(), String> {
        match self.0.keys().find(|field| !known.contains(&field.as_str())) {
            Some(field) => Err(format!("{field} is not a field of {request}")),
            None => Ok(()),
        }
    }

    /// # Errors
    /// The field is missing or is not text.
    pub fn required_text(&self, name: &str) -> Result<String, String> {
        self.optional_text(name)?
            .ok_or_else(|| format!("{name} is required"))
    }

    /// # Errors
    /// The field is present but is not text. A `null` counts as absent.
    pub fn optional_text(&self, name: &str) -> Result<Option<String>, String> {
        match self.0.get(name) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(text)) => Ok(Some(text.clone())),
            Some(_) => Err(format!("{name} must be text")),
        }
    }

    /// # Errors
    /// The field is present but is not a boolean.
    pub fn optional_bool(&self, name: &str) -> Result<Option<bool>, String> {
        match self.0.get(name) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Bool(value)) => Ok(Some(*value)),
            Some(_) => Err(format!("{name} must be a boolean")),
        }
    }
}

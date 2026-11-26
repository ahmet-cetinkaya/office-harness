// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::{Map, Value};

/// The body of a request, read field by field so that every failure names the
/// field.
pub struct Fields<'a>(pub &'a Map<String, Value>);

impl Fields<'_> {
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
    /// The field is missing or is not true or false.
    pub fn required_bool(&self, name: &str) -> Result<bool, String> {
        match self.0.get(name) {
            Some(Value::Bool(flag)) => Ok(*flag),
            _ => Err(format!("{name} must be true or false")),
        }
    }

    /// # Errors
    /// The field is missing or is not an object.
    pub fn required_object(&self, name: &str) -> Result<&Map<String, Value>, String> {
        match self.0.get(name) {
            Some(Value::Object(fields)) => Ok(fields),
            _ => Err(format!("{name} must be an object")),
        }
    }
}

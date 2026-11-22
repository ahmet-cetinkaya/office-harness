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
    /// The field is present but is not a list of text.
    pub fn optional_text_list(&self, name: &str) -> Result<Option<Vec<String>>, String> {
        let list = match self.0.get(name) {
            None | Some(Value::Null) => return Ok(None),
            Some(Value::Array(list)) => list,
            Some(_) => return Err(format!("{name} must be a list of text")),
        };
        list.iter()
            .map(|item| {
                item.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| format!("{name} must be a list of text"))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some)
    }

    /// # Errors
    /// The field is missing or is not an object.
    pub fn required_object(&self, name: &str) -> Result<&Map<String, Value>, String> {
        match self.0.get(name) {
            Some(Value::Object(fields)) => Ok(fields),
            _ => Err(format!("{name} must be an object")),
        }
    }

    /// # Errors
    /// The field is present but is not an object. A `null` counts as absent.
    pub fn optional_object(&self, name: &str) -> Result<Option<&Map<String, Value>>, String> {
        match self.0.get(name) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Object(fields)) => Ok(Some(fields)),
            Some(_) => Err(format!("{name} must be an object")),
        }
    }
}

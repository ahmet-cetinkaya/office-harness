// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::{Map, Value};

/// The body of a request, read field by field so that every failure names the
/// field. (A derived reader says "invalid type" without the name.)
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
}

#[cfg(test)]
#[path = "fields.tests.rs"]
mod tests;

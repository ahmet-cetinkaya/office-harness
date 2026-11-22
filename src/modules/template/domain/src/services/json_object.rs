// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::{Map, Value};

use crate::errors::TemplateError;

/// A JSON object whose keys are all known: the reader that makes every
/// parser refuse an unknown field and name it (D2).
pub struct KnownObject<'a> {
    path: String,
    fields: &'a Map<String, Value>,
}

impl<'a> KnownObject<'a> {
    /// # Errors
    /// `WrongShape` when `value` is not an object and `UnknownField` for the
    /// first key outside `known`.
    pub fn read(
        value: &'a Value,
        path: &str,
        what: &'static str,
        known: &[&str],
    ) -> Result<Self, TemplateError> {
        let Value::Object(fields) = value else {
            return Err(TemplateError::WrongShape {
                path: path.to_owned(),
                expected: "an object",
            });
        };
        if let Some(unknown) = fields.keys().find(|key| !known.contains(&key.as_str())) {
            return Err(TemplateError::UnknownField {
                path: child_path(path, unknown),
                what,
            });
        }
        Ok(Self {
            path: path.to_owned(),
            fields,
        })
    }

    #[must_use]
    pub fn path_of(&self, field: &str) -> String {
        child_path(&self.path, field)
    }

    /// The field, or `None` when it is absent or null.
    #[must_use]
    pub fn optional(&self, field: &str) -> Option<&'a Value> {
        self.fields.get(field).filter(|value| !value.is_null())
    }

    /// # Errors
    /// `Missing` when the field is absent or null.
    pub fn required(&self, field: &str) -> Result<&'a Value, TemplateError> {
        self.optional(field).ok_or_else(|| TemplateError::Missing {
            path: self.path_of(field),
        })
    }

    /// # Errors
    /// `Missing`, or `WrongShape` when the field is not text.
    pub fn required_text(&self, field: &str) -> Result<&'a str, TemplateError> {
        text_of(self.required(field)?, &self.path_of(field))
    }

    /// # Errors
    /// `WrongShape` when the field is present and not text.
    pub fn optional_text(&self, field: &str) -> Result<Option<&'a str>, TemplateError> {
        self.optional(field)
            .map(|value| text_of(value, &self.path_of(field)))
            .transpose()
    }

    /// # Errors
    /// `WrongShape` when the field is present and not a list.
    pub fn optional_list(&self, field: &str) -> Result<&'a [Value], TemplateError> {
        match self.optional(field) {
            None => Ok(&[]),
            Some(Value::Array(items)) => Ok(items),
            Some(_) => Err(TemplateError::WrongShape {
                path: self.path_of(field),
                expected: "a list",
            }),
        }
    }
}

/// # Errors
/// `WrongShape` when `value` is not text.
pub fn text_of<'a>(value: &'a Value, path: &str) -> Result<&'a str, TemplateError> {
    value.as_str().ok_or_else(|| TemplateError::WrongShape {
        path: path.to_owned(),
        expected: "text",
    })
}

fn child_path(parent: &str, field: &str) -> String {
    format!("{parent}.{field}")
}

#[must_use]
pub fn item_path(parent: &str, index: usize) -> String {
    format!("{parent}[{index}]")
}

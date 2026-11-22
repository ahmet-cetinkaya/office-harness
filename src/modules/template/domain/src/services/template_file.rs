// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::{Value, json};

use super::body_json::{body_to_json, parse_body};
use super::json_object::{KnownObject, item_path};
use crate::entities::TemplateBody;
use crate::enums::TemplateKind;
use crate::errors::TemplateError;
use crate::value_objects::TemplateName;

const FILE_FORMAT: &str = "office-harness-templates";
const FILE_VERSION: u64 = 1;
const FILE_ROOT: &str = "file";
const FILE_FIELDS: &[&str] = &["format", "version", "templates"];
const TEMPLATE_FIELDS: &[&str] = &["name", "kind", "body"];
const BYTES_PER_KIB: usize = 1024;

/// The largest template file the office reads (FR-009). It is far above any
/// real file; the limit keeps an untrusted file from filling memory.
pub const MAX_FILE_BYTES: usize = 256 * BYTES_PER_KIB;

/// One template as a file holds it: a name and a body, no id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTemplate {
    pub name: TemplateName,
    pub body: TemplateBody,
}

/// Reads a template file (FR-008, FR-009). The whole file is refused at the
/// first template that breaks a rule, and the refusal names that template.
/// Environment values are read as they are; the caller drops them (D3).
///
/// # Errors
/// `FileTooLarge`, `NotATemplateFile`, `UnsupportedFileVersion`,
/// `NoTemplatesInFile`, `UnknownField`, or `InTemplate` around the rule that
/// one template breaks.
pub fn parse_file(text: &str) -> Result<Vec<FileTemplate>, TemplateError> {
    if text.len() > MAX_FILE_BYTES {
        return Err(TemplateError::FileTooLarge {
            limit_kib: MAX_FILE_BYTES / BYTES_PER_KIB,
        });
    }
    let document: Value =
        serde_json::from_str(text).map_err(|error| TemplateError::NotATemplateFile {
            detail: error.to_string(),
        })?;
    let object = read_file_object(&document)?;
    object
        .optional_list("templates")?
        .iter()
        .enumerate()
        .map(|(index, template)| read_template(template, index))
        .collect::<Result<Vec<_>, _>>()
        .and_then(|templates| {
            if templates.is_empty() {
                Err(TemplateError::NoTemplatesInFile)
            } else {
                Ok(templates)
            }
        })
}

fn read_file_object(document: &Value) -> Result<KnownObject<'_>, TemplateError> {
    let Some(fields) = document.as_object() else {
        return Err(not_a_template_file("the text is not an object"));
    };
    if fields.get("format").and_then(Value::as_str) != Some(FILE_FORMAT) {
        return Err(not_a_template_file(&format!(
            "format is not \"{FILE_FORMAT}\""
        )));
    }
    let version = fields.get("version").cloned().unwrap_or(Value::Null);
    if version.as_u64() != Some(FILE_VERSION) {
        return Err(TemplateError::UnsupportedFileVersion {
            found: version.to_string(),
            supported: FILE_VERSION,
        });
    }
    KnownObject::read(document, FILE_ROOT, "a template file", FILE_FIELDS)
}

fn not_a_template_file(detail: &str) -> TemplateError {
    TemplateError::NotATemplateFile {
        detail: detail.to_owned(),
    }
}

fn read_template(value: &Value, index: usize) -> Result<FileTemplate, TemplateError> {
    let label = value
        .get("name")
        .and_then(Value::as_str)
        .map_or_else(|| format!("template {}", index + 1), str::to_owned);
    read_checked_template(value, index).map_err(|source| TemplateError::InTemplate {
        template: label,
        source: Box::new(source),
    })
}

fn read_checked_template(value: &Value, index: usize) -> Result<FileTemplate, TemplateError> {
    let path = item_path("templates", index);
    let object = KnownObject::read(value, &path, "a template", TEMPLATE_FIELDS)?;
    let name = TemplateName::parse(object.required_text("name")?)?;
    let kind = TemplateKind::parse(object.required_text("kind")?)?;
    let body = parse_body(kind, object.required("body")?)?;
    body.validate()?;
    Ok(FileTemplate { name, body })
}

/// The file text of `templates`, with every environment value `null`
/// (FR-007, A6).
#[must_use]
pub fn render_file(templates: &[FileTemplate]) -> String {
    let entries: Vec<Value> = templates
        .iter()
        .map(|template| {
            let body = template.body.clone().without_env_values();
            json!({
                "name": template.name.as_str(),
                "kind": body.kind().as_str(),
                "body": body_to_json(&body),
            })
        })
        .collect();
    let document = json!({
        "format": FILE_FORMAT,
        "version": FILE_VERSION,
        "templates": entries,
    });
    format!("{document:#}")
}

#[cfg(test)]
#[path = "template_file.tests.rs"]
mod tests;

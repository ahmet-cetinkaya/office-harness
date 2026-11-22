// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use kernel::Timestamp;
use rusqlite::ffi::{ErrorCode, SQLITE_CONSTRAINT_UNIQUE};
use rusqlite::{Connection, OptionalExtension, Row, Transaction, params};
use template_application::common::errors::TemplateApplicationError;
use template_application::common::interfaces::TemplateStore;
use template_domain::{
    Template, TemplateBody, TemplateId, TemplateKind, TemplateName, body_to_json, parse_body,
};

/// One shared SQLite connection, the way the host app hands the database to
/// every module (each module keeps its own tables).
pub type SharedHandle = Arc<Mutex<Connection>>;

const SELECT_TEMPLATE: &str = "SELECT id, name, kind, body, updated_at FROM template_templates";
const INSERT_TEMPLATE: &str = "INSERT INTO template_templates \
                               (id, name, name_key, kind, body, updated_at) \
                               VALUES (?1, ?2, ?3, ?4, ?5, ?6)";

struct TemplateRow {
    id: String,
    name: String,
    kind: String,
    body: String,
    updated_at: i64,
}

impl TemplateRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            kind: row.get(2)?,
            body: row.get(3)?,
            updated_at: row.get(4)?,
        })
    }

    /// The stored text is checked again by the domain rules, so a damaged row
    /// cannot give an invalid template to a creation.
    fn into_template(self) -> Result<Template, TemplateApplicationError> {
        let id = self
            .id
            .parse::<TemplateId>()
            .map_err(|error| corrupt("template id", &error.to_string()))?;
        let name = TemplateName::parse(&self.name)
            .map_err(|error| corrupt("template name", &error.to_string()))?;
        let kind = TemplateKind::parse(&self.kind)
            .map_err(|error| corrupt("template kind", &error.to_string()))?;
        let body = serde_json::from_str(&self.body)
            .map_err(|error| corrupt("template body", &error.to_string()))
            .and_then(|json| {
                parse_body(kind, &json)
                    .map_err(|error| corrupt("template body", &error.to_string()))
            })?;
        Template::new(id, name, body, Timestamp::from_unix_millis(self.updated_at))
            .map_err(|error| corrupt("template body", &error.to_string()))
    }
}

fn corrupt(field: &'static str, text: &str) -> TemplateApplicationError {
    TemplateApplicationError::Storage {
        message: format!("stored data is invalid, {field}: {text}"),
        is_disk_failure: false,
    }
}

fn storage_failure(error: &rusqlite::Error) -> TemplateApplicationError {
    let is_disk_failure = matches!(
        error.sqlite_error_code(),
        Some(
            ErrorCode::DiskFull
                | ErrorCode::SystemIoFailure
                | ErrorCode::ReadOnly
                | ErrorCode::CannotOpen
        )
    );
    TemplateApplicationError::Storage {
        message: error.to_string(),
        is_disk_failure,
    }
}

/// A unique-index failure is a name that another template holds; any other
/// failure is a storage failure.
fn write_failure(error: &rusqlite::Error, template: &Template) -> TemplateApplicationError {
    if matches!(
        error,
        rusqlite::Error::SqliteFailure(failure, _) if failure.extended_code == SQLITE_CONSTRAINT_UNIQUE
    ) {
        return TemplateApplicationError::NameTaken {
            name: template.name.to_string(),
        };
    }
    storage_failure(error)
}

/// The templates in the `template_templates` table.
#[derive(Clone)]
pub struct TemplateRepository {
    connection: SharedHandle,
}

impl TemplateRepository {
    #[must_use]
    pub fn new(connection: SharedHandle) -> Self {
        Self { connection }
    }

    fn connection(&self) -> MutexGuard<'_, Connection> {
        // A panic in another call leaves SQLite consistent, so a poisoned
        // lock is safe to reuse.
        self.connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

fn insert_row(
    connection: &Connection,
    template: &Template,
) -> Result<(), TemplateApplicationError> {
    connection
        .execute(
            INSERT_TEMPLATE,
            params![
                template.id.to_string(),
                template.name.as_str(),
                template.name.key(),
                template.body.kind().as_str(),
                body_text(&template.body),
                template.updated_at.as_unix_millis(),
            ],
        )
        .map_err(|error| write_failure(&error, template))?;
    Ok(())
}

fn body_text(body: &TemplateBody) -> String {
    body_to_json(body).to_string()
}

impl TemplateStore for TemplateRepository {
    fn list(&self) -> Result<Vec<Template>, TemplateApplicationError> {
        let connection = self.connection();
        let mut statement = connection
            .prepare(&format!("{SELECT_TEMPLATE} ORDER BY name_key"))
            .map_err(|error| storage_failure(&error))?;
        let rows = statement
            .query_map([], TemplateRow::read)
            .map_err(|error| storage_failure(&error))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| storage_failure(&error))?;
        rows.into_iter().map(TemplateRow::into_template).collect()
    }

    fn find(&self, id: TemplateId) -> Result<Option<Template>, TemplateApplicationError> {
        let row = self
            .connection()
            .query_row(
                &format!("{SELECT_TEMPLATE} WHERE id = ?1"),
                params![id.to_string()],
                TemplateRow::read,
            )
            .optional()
            .map_err(|error| storage_failure(&error))?;
        row.map(TemplateRow::into_template).transpose()
    }

    fn insert(&self, template: &Template) -> Result<(), TemplateApplicationError> {
        insert_row(&self.connection(), template)
    }

    fn update(&self, template: &Template) -> Result<(), TemplateApplicationError> {
        let changed = self
            .connection()
            .execute(
                "UPDATE template_templates \
                 SET name = ?2, name_key = ?3, kind = ?4, body = ?5, updated_at = ?6 \
                 WHERE id = ?1",
                params![
                    template.id.to_string(),
                    template.name.as_str(),
                    template.name.key(),
                    template.body.kind().as_str(),
                    body_text(&template.body),
                    template.updated_at.as_unix_millis(),
                ],
            )
            .map_err(|error| write_failure(&error, template))?;
        if changed == 0 {
            return Err(TemplateApplicationError::NotFound(format!(
                "template {} does not exist",
                template.id
            )));
        }
        Ok(())
    }

    fn delete(&self, id: TemplateId) -> Result<bool, TemplateApplicationError> {
        let deleted = self
            .connection()
            .execute(
                "DELETE FROM template_templates WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|error| storage_failure(&error))?;
        Ok(deleted > 0)
    }

    fn insert_all(&self, templates: &[Template]) -> Result<(), TemplateApplicationError> {
        let mut connection = self.connection();
        let transaction: Transaction<'_> = connection
            .transaction()
            .map_err(|error| storage_failure(&error))?;
        for template in templates {
            insert_row(&transaction, template)?;
        }
        transaction
            .commit()
            .map_err(|error| storage_failure(&error))
    }
}

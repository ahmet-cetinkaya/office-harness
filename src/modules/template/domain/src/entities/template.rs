// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::Timestamp;

use crate::entities::TemplateBody;
use crate::errors::TemplateError;
use crate::value_objects::{TemplateId, TemplateName};

/// A saved agent or team definition. It is a copy: no agent or team created
/// from it keeps a link to it (FR-006).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    pub id: TemplateId,
    pub name: TemplateName,
    pub body: TemplateBody,
    pub updated_at: Timestamp,
}

impl Template {
    /// # Errors
    /// The save rule that the body breaks.
    pub fn new(
        id: TemplateId,
        name: TemplateName,
        body: TemplateBody,
        updated_at: Timestamp,
    ) -> Result<Self, TemplateError> {
        body.validate()?;
        Ok(Self {
            id,
            name,
            body,
            updated_at,
        })
    }
}

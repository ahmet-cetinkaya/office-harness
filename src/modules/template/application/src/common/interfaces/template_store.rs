// SPDX-License-Identifier: AGPL-3.0-or-later

use template_domain::{Template, TemplateId};

use crate::common::errors::TemplateApplicationError;

/// Where templates are kept (A8). Names are unique case-insensitively.
pub trait TemplateStore: Send + Sync {
    /// Every template, ordered by name ignoring case.
    ///
    /// # Errors
    /// `Storage`, also for a stored row that the domain rejects.
    fn list(&self) -> Result<Vec<Template>, TemplateApplicationError>;

    /// # Errors
    /// `Storage`.
    fn find(&self, id: TemplateId) -> Result<Option<Template>, TemplateApplicationError>;

    /// # Errors
    /// `NameTaken` when another template has the name, `Storage`.
    fn insert(&self, template: &Template) -> Result<(), TemplateApplicationError>;

    /// # Errors
    /// `NameTaken`, `NotFound` when the template does not exist, `Storage`.
    fn update(&self, template: &Template) -> Result<(), TemplateApplicationError>;

    /// Whether a template was deleted.
    ///
    /// # Errors
    /// `Storage`.
    fn delete(&self, id: TemplateId) -> Result<bool, TemplateApplicationError>;

    /// Saves every template or none of them, in one transaction.
    ///
    /// # Errors
    /// `NameTaken`, `Storage`.
    fn insert_all(&self, templates: &[Template]) -> Result<(), TemplateApplicationError>;
}

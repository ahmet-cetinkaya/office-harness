// SPDX-License-Identifier: AGPL-3.0-or-later

use template_domain::{TemplateBody, TemplateName};

/// One template of an import file, as the preview shows it (FR-008): its
/// name in the file, the name it will be saved under (A3), and its body with
/// every environment value dropped (D3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportPreviewEntry {
    pub name: TemplateName,
    pub save_name: TemplateName,
    pub body: TemplateBody,
}

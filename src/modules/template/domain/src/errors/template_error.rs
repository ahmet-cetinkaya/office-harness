// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;

/// Why a template, a body, or a template file is refused. Every text names
/// the part at fault, so the user sees what to change.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TemplateError {
    #[error("the template name is empty")]
    NameEmpty,
    #[error("the template name is longer than {max} characters")]
    NameTooLong { max: usize },
    #[error("the template name has a control character")]
    NameHasControlCharacter,
    #[error("{place}: the CLI command is empty")]
    CommandEmpty { place: String },
    #[error("{place}: the name pattern is empty")]
    PatternEmpty { place: String },
    #[error("{place}: the name pattern \"{pattern}\" makes a name longer than {max} characters")]
    PatternTooLong {
        place: String,
        pattern: String,
        max: usize,
    },
    #[error("members[{index}]: the count is below 1")]
    CountBelowOne { index: usize },
    #[error("a team template needs at least one member entry")]
    NoMembers,
    #[error("the team would create {total} agents; the limit is {max}")]
    TooManyAgents { total: u64, max: u64 },
    #[error("{path} is not a field of {what}")]
    UnknownField { path: String, what: &'static str },
    #[error("{path} is required")]
    Missing { path: String },
    #[error("{path} must be {expected}")]
    WrongShape {
        path: String,
        expected: &'static str,
    },
    #[error("the body version {found} is not supported; this build reads version {supported}")]
    UnsupportedBodyVersion { found: String, supported: u64 },
    #[error("the kind \"{kind}\" is not \"agent\" or \"team\"")]
    UnknownKind { kind: String },
    #[error("the role \"{role}\" is not \"ceo\", \"lead\", or \"member\"")]
    UnknownRole { role: String },
    #[error("the template id is not a valid UUID: {text}")]
    InvalidId { text: String },
    #[error("the file is larger than {limit_kib} KiB")]
    FileTooLarge { limit_kib: usize },
    #[error("the file is not a template file: {detail}")]
    NotATemplateFile { detail: String },
    #[error("the file version {found} is not supported; this build reads version {supported}")]
    UnsupportedFileVersion { found: String, supported: u64 },
    #[error("the file holds no template")]
    NoTemplatesInFile,
    #[error("template \"{template}\": {source}")]
    InTemplate {
        template: String,
        source: Box<TemplateError>,
    },
}

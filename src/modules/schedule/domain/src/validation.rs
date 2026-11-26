// SPDX-License-Identifier: AGPL-3.0-or-later

use icu_normalizer::ComposingNormalizerBorrowed;
use jiff::tz::TimeZone;
use kernel::Timestamp;
use thiserror::Error;

use crate::Repeat;
use crate::due::local_due;

pub const NAME_MAX_CHARS: usize = 100;
pub const TEXT_MAX_CHARS: usize = 8192;
pub const MIN_HOURS: u32 = 1;
pub const MAX_HOURS: u32 = 168;

pub const NAME_FIELD: &str = "name";
pub const TEXT_FIELD: &str = "text";
pub const REPEAT_FIELD: &str = "repeat";
pub const TARGET_FIELD: &str = "target";

/// The name as the office keeps it: without the spaces at its ends and in
/// Unicode normalization form C, so "Nightly" and "Nightly " are one name and
/// a letter with an accent has one spelling.
#[must_use]
pub fn normalize_name(name: &str) -> String {
    ComposingNormalizerBorrowed::new_nfc()
        .normalize(name.trim())
        .into_owned()
}

/// The rule that a field broke.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum Rule {
    #[error("must not be blank")]
    Blank,
    #[error("must have at most {max} characters")]
    TooLong { max: usize },
    #[error("must not have control characters")]
    ControlCharacter,
    #[error("must not have invisible or direction-changing characters")]
    FormatCharacter,
    #[error("a weekly repeat needs at least one day")]
    NoDay,
    #[error("hours must be a whole number from {MIN_HOURS} to {MAX_HOURS}")]
    HoursOutOfRange,
    #[error("a once repeat must name a future local time")]
    NotInFuture,
    #[error("a time must be hours and minutes, HH:MM")]
    BadTime,
    #[error("a date must be a real calendar date, YYYY-MM-DD")]
    BadDate,
    #[error("is used by another schedule")]
    NameTaken,
    #[error("must name an agent that exists")]
    TargetMissing,
}

/// One failing field and the rule it broke.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldError {
    pub field: &'static str,
    pub rule: Rule,
}

/// Checks every field of a schedule that the person saves and returns each
/// failure, in field order; empty means the fields are acceptable. Whether
/// another schedule has the same name is the store's check, not this one.
/// A once time must be in the future only while the schedule is on.
#[must_use]
pub fn validate_fields(
    name: &str,
    text: &str,
    repeat: &Repeat,
    switched_on: bool,
    now: Timestamp,
    zone: &TimeZone,
) -> Vec<FieldError> {
    let mut errors = Vec::new();
    check_text(NAME_FIELD, name, NAME_MAX_CHARS, &mut errors);
    if !errors.iter().any(|error| error.field == NAME_FIELD) && name.chars().any(is_format_char) {
        errors.push(FieldError {
            field: NAME_FIELD,
            rule: Rule::FormatCharacter,
        });
    }
    check_text(TEXT_FIELD, text, TEXT_MAX_CHARS, &mut errors);
    if let Some(rule) = repeat_rule(repeat, switched_on, now, zone) {
        errors.push(FieldError {
            field: REPEAT_FIELD,
            rule,
        });
    }
    errors
}

/// The characters that show nothing or change the direction of the text around
/// them, such as the zero-width space and the bidi controls. A name with one
/// would look the same as another name, or reorder the name next to it. A
/// directive's text may hold them (an emoji sequence needs the zero-width
/// joiner), so only the name is checked.
const FORMAT_RANGES: [(char, char); 14] = [
    ('\u{00AD}', '\u{00AD}'),
    ('\u{0600}', '\u{0605}'),
    ('\u{061C}', '\u{061C}'),
    ('\u{06DD}', '\u{06DD}'),
    ('\u{070F}', '\u{070F}'),
    ('\u{180E}', '\u{180E}'),
    ('\u{200B}', '\u{200F}'),
    ('\u{202A}', '\u{202E}'),
    ('\u{2060}', '\u{2064}'),
    ('\u{2066}', '\u{206F}'),
    ('\u{FEFF}', '\u{FEFF}'),
    ('\u{FFF9}', '\u{FFFB}'),
    ('\u{E0001}', '\u{E0001}'),
    ('\u{E0020}', '\u{E007F}'),
];

fn is_format_char(character: char) -> bool {
    FORMAT_RANGES
        .iter()
        .any(|&(first, last)| (first..=last).contains(&character))
}

// A line break counts as a control character here, as in a directive's text.
fn check_text(field: &'static str, value: &str, max: usize, errors: &mut Vec<FieldError>) {
    let rule = if value.trim().is_empty() {
        Rule::Blank
    } else if value.chars().count() > max {
        Rule::TooLong { max }
    } else if value.chars().any(char::is_control) {
        Rule::ControlCharacter
    } else {
        return;
    };
    errors.push(FieldError { field, rule });
}

fn repeat_rule(
    repeat: &Repeat,
    switched_on: bool,
    now: Timestamp,
    zone: &TimeZone,
) -> Option<Rule> {
    match repeat {
        Repeat::Weekly { days, .. } if days.is_empty() => Some(Rule::NoDay),
        Repeat::EveryHours { hours } if !(MIN_HOURS..=MAX_HOURS).contains(hours) => {
            Some(Rule::HoursOutOfRange)
        }
        Repeat::Once { date, time } if switched_on => {
            let is_future = local_due(zone, *date, *time).is_some_and(|due| due > now);
            (!is_future).then_some(Rule::NotInFuture)
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "validation.tests.rs"]
mod tests;

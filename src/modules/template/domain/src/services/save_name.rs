// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::value_objects::TemplateName;

const FIRST_NUMBER_AFTER_THE_PLAIN_NAME: u32 = 2;

/// The name an imported template is saved under (A3): its own name, or the
/// name with ` (2)`, ` (3)`, and so on, the lowest one whose key is free.
/// `is_key_taken` gets the lower-cased key.
pub fn save_name(name: &TemplateName, is_key_taken: impl Fn(&str) -> bool) -> TemplateName {
    if !is_key_taken(&name.key()) {
        return name.clone();
    }
    let mut number = FIRST_NUMBER_AFTER_THE_PLAIN_NAME;
    loop {
        let candidate = name.numbered(number);
        if !is_key_taken(&candidate.key()) {
            return candidate;
        }
        number += 1;
    }
}

#[cfg(test)]
#[path = "save_name.tests.rs"]
mod tests;

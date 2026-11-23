// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, RwLock};

use kernel::{StreamRedactor, TextRedactor};
use secret_domain::{SecretName, SecretValue};
use zeroize::Zeroizing;

/// One value to replace and the placeholder that stands for it.
struct Pattern {
    value: Zeroizing<Vec<u8>>,
    placeholder: Vec<u8>,
}

type Patterns = Arc<Vec<Pattern>>;

/// Replaces the exact value of every secret by `[secret NAME]` in the text and
/// the output that the office stores or sends (0026/FR-012). It holds the
/// values open in memory, in buffers that clear when they drop; `replace_all`
/// swaps them at each change. The longest value is tried first, so a value
/// that holds another one is replaced whole.
#[derive(Default)]
pub struct SecretRedactor {
    patterns: Arc<RwLock<Patterns>>,
}

impl SecretRedactor {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the values the redactor knows.
    pub fn replace_all(&self, values: Vec<(SecretName, SecretValue)>) {
        let mut patterns: Vec<Pattern> = values
            .into_iter()
            .map(|(name, value)| Pattern {
                value: Zeroizing::new(value.reveal().as_bytes().to_vec()),
                placeholder: format!("[secret {name}]").into_bytes(),
            })
            .collect();
        patterns.sort_by_key(|pattern| std::cmp::Reverse(pattern.value.len()));
        *self
            .patterns
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Arc::new(patterns);
    }

    #[must_use]
    pub fn knows_no_value(&self) -> bool {
        current(&self.patterns).is_empty()
    }
}

fn current(patterns: &RwLock<Patterns>) -> Patterns {
    Arc::clone(
        &patterns
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    )
}

impl TextRedactor for SecretRedactor {
    fn redact_text(&self, text: &str) -> String {
        let (replaced, _) = scan(&current(&self.patterns), text.as_bytes(), true);
        // A value and a placeholder are whole UTF-8 text, so a replacement
        // never cuts a character.
        String::from_utf8(replaced).unwrap_or_else(|_| text.to_owned())
    }

    fn redact_cut_text(&self, text: &str) -> String {
        let (replaced, begun) = scan(&current(&self.patterns), text.as_bytes(), false);
        // What "begun" holds is the start of a value that the cut may have
        // shortened: it is dropped, not kept.
        drop(begun);
        String::from_utf8(replaced).unwrap_or_default()
    }

    fn stream(&self) -> Box<dyn StreamRedactor> {
        Box::new(SecretStream {
            patterns: Arc::clone(&self.patterns),
            held: Vec::new(),
        })
    }
}

/// One output stream. The end of a chunk that could be the start of a value
/// waits for the next chunk.
struct SecretStream {
    patterns: Arc<RwLock<Patterns>>,
    held: Vec<u8>,
}

impl StreamRedactor for SecretStream {
    fn push(&mut self, bytes: &[u8]) -> Vec<u8> {
        let patterns = current(&self.patterns);
        if patterns.is_empty() && self.held.is_empty() {
            return bytes.to_vec();
        }
        let mut input = std::mem::take(&mut self.held);
        input.extend_from_slice(bytes);
        let (output, held) = scan(&patterns, &input, false);
        self.held = held;
        output
    }

    fn finish(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.held)
    }
}

/// Replaces each value in `input`. Unless `at_end`, the rest of the input
/// that is a proper start of some value is not output but returned as held.
fn scan(patterns: &[Pattern], input: &[u8], at_end: bool) -> (Vec<u8>, Vec<u8>) {
    let mut output = Vec::with_capacity(input.len());
    let mut at = 0;
    while at < input.len() {
        let rest = &input[at..];
        if let Some(pattern) = patterns
            .iter()
            .find(|pattern| rest.starts_with(&pattern.value))
        {
            output.extend_from_slice(&pattern.placeholder);
            at += pattern.value.len();
            continue;
        }
        if !at_end && starts_a_value(patterns, rest) {
            return (output, rest.to_vec());
        }
        output.push(input[at]);
        at += 1;
    }
    (output, Vec::new())
}

fn starts_a_value(patterns: &[Pattern], rest: &[u8]) -> bool {
    patterns
        .iter()
        .any(|pattern| rest.len() < pattern.value.len() && pattern.value.starts_with(rest))
}

#[cfg(test)]
#[path = "redactor.tests.rs"]
mod tests;

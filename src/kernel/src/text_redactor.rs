// SPDX-License-Identifier: AGPL-3.0-or-later

/// Replaces what must not be stored or sent, in text and in a stream of
/// output bytes (0026/FR-012). A module takes the trait, so it never names the
/// module that knows what to replace.
pub trait TextRedactor: Send + Sync {
    /// The text with each occurrence replaced. Text with nothing to replace
    /// comes back unchanged.
    fn redact_text(&self, text: &str) -> String;

    /// For text that was cut from a longer one, as a command line is cut to
    /// a display length: a tail that begins an occurrence is dropped, because
    /// the cut may have taken the rest of it.
    fn redact_cut_text(&self, text: &str) -> String {
        self.redact_text(text)
    }

    /// A redactor for one stream of output, which keeps what it needs between
    /// chunks, so an occurrence split across two chunks is still replaced.
    fn stream(&self) -> Box<dyn StreamRedactor>;
}

/// One output stream. It may hold back the end of a chunk while that end could
/// be the start of an occurrence; `finish` gives it up, at the end of the
/// stream and when no more output comes for a short time.
pub trait StreamRedactor: Send {
    fn push(&mut self, bytes: &[u8]) -> Vec<u8>;

    /// What `push` still holds back.
    fn finish(&mut self) -> Vec<u8>;
}

/// Replaces nothing.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoRedaction;

impl TextRedactor for NoRedaction {
    fn redact_text(&self, text: &str) -> String {
        text.to_owned()
    }

    fn stream(&self) -> Box<dyn StreamRedactor> {
        Box::new(PassThrough)
    }
}

struct PassThrough;

impl StreamRedactor for PassThrough {
    fn push(&mut self, bytes: &[u8]) -> Vec<u8> {
        bytes.to_vec()
    }

    fn finish(&mut self) -> Vec<u8> {
        Vec::new()
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;
use std::sync::Arc;

use agent_application::common::interfaces::{CliAdapter, CliAdapters};

use super::claude::ClaudeAdapter;
use super::codex::CodexAdapter;
use super::codex_sessions::AttributionTiming;
use super::gemini::GeminiAdapter;
use super::generic::GenericAdapter;
use super::opencode::OpencodeAdapter;

/// Chooses the adapter by the program's name: `claude`, `codex`, `gemini`,
/// or `opencode`, without folder, extension, or case. Any other program gets
/// the generic adapter.
pub struct CliAdapterRegistry {
    claude: Arc<dyn CliAdapter>,
    codex: Arc<dyn CliAdapter>,
    gemini: Arc<dyn CliAdapter>,
    opencode: Arc<dyn CliAdapter>,
    generic: Arc<dyn CliAdapter>,
}

impl CliAdapterRegistry {
    #[must_use]
    pub fn new(codex_timing: AttributionTiming) -> Self {
        Self {
            claude: Arc::new(ClaudeAdapter),
            codex: Arc::new(CodexAdapter::new(codex_timing)),
            gemini: Arc::new(GeminiAdapter),
            opencode: Arc::new(OpencodeAdapter),
            generic: Arc::new(GenericAdapter),
        }
    }
}

impl CliAdapters for CliAdapterRegistry {
    fn for_program(&self, program: &str) -> Arc<dyn CliAdapter> {
        let name = Path::new(program)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(str::to_ascii_lowercase);
        let adapter = match name.as_deref() {
            Some("claude") => &self.claude,
            Some("codex") => &self.codex,
            Some("gemini") => &self.gemini,
            Some("opencode") => &self.opencode,
            _ => &self.generic,
        };
        Arc::clone(adapter)
    }
}

#[cfg(test)]
#[path = "registry.tests.rs"]
mod tests;

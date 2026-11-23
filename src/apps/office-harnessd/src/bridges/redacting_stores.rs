// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_application::common::errors::PortError;
use agent_application::common::interfaces::SessionProcessStore;
use agent_application::common::models::{ProcessKind, SessionProcess, StoredProcess};
use agent_domain::value_objects::SessionId;
use audit_application::common::errors::AuditError;
use audit_application::common::interfaces::ActionLog;
use audit_domain::{ActionEntry, ActionRecord, ActionResult};
use kernel::{AgentId, TextRedactor};

/// Replaces a secret value in the display text of a stored process identity
/// before it is stored (0026/FR-012 c). The hash of the whole command line
/// comes with the identity unchanged: it was computed from the original, so
/// the sweep still matches the process (PB-8).
pub struct RedactingProcessStore {
    inner: Arc<dyn SessionProcessStore>,
    redactor: Arc<dyn TextRedactor>,
}

impl RedactingProcessStore {
    #[must_use]
    pub fn new(inner: Arc<dyn SessionProcessStore>, redactor: Arc<dyn TextRedactor>) -> Self {
        Self { inner, redactor }
    }

    /// The display text is cut from a longer command line, so a tail that
    /// begins a value is dropped.
    fn redacted(&self, process: &SessionProcess) -> SessionProcess {
        SessionProcess {
            command_display: self.redactor.redact_cut_text(&process.command_display),
            ..process.clone()
        }
    }
}

impl SessionProcessStore for RedactingProcessStore {
    fn insert_open(
        &self,
        session_id: SessionId,
        kind: ProcessKind,
        process: &SessionProcess,
    ) -> Result<bool, PortError> {
        self.inner
            .insert_open(session_id, kind, &self.redacted(process))
    }

    fn open_for_session(&self, session_id: SessionId) -> Result<Vec<StoredProcess>, PortError> {
        self.inner.open_for_session(session_id)
    }

    fn update_command(&self, id: i64, process: &SessionProcess) -> Result<(), PortError> {
        self.inner.update_command(id, &self.redacted(process))
    }

    fn drop_helper(&self, id: i64) -> Result<(), PortError> {
        self.inner.drop_helper(id)
    }

    fn mark_helper_limit(&self, session_id: SessionId) -> Result<(), PortError> {
        self.inner.mark_helper_limit(session_id)
    }
}

/// The last net under the action record: every text that reaches it, the
/// detail and the reason of a failure, loses a secret value first
/// (0026/FR-012 b). The texts come from many places, a refusal that quotes a
/// field name among them.
pub struct RedactingActionLog {
    inner: Arc<dyn ActionLog>,
    redactor: Arc<dyn TextRedactor>,
}

impl RedactingActionLog {
    #[must_use]
    pub fn new(inner: Arc<dyn ActionLog>, redactor: Arc<dyn TextRedactor>) -> Self {
        Self { inner, redactor }
    }
}

impl ActionLog for RedactingActionLog {
    fn append(&self, record: &ActionRecord) -> Result<ActionEntry, AuditError> {
        let result = match &record.result {
            ActionResult::Failed(reason) => ActionResult::Failed(self.redactor.redact_text(reason)),
            ActionResult::Ok => ActionResult::Ok,
        };
        self.inner.append(&ActionRecord {
            result,
            detail: record
                .detail
                .as_deref()
                .map(|text| self.redactor.redact_text(text)),
            ..record.clone()
        })
    }

    fn list_for_agent(&self, agent_id: AgentId) -> Result<Vec<ActionEntry>, AuditError> {
        self.inner.list_for_agent(agent_id)
    }

    fn list_office_level(&self) -> Result<Vec<ActionEntry>, AuditError> {
        self.inner.list_office_level()
    }
}

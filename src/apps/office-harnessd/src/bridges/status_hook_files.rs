// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use access_application::common::interfaces::HookSecretStore;
use agent_application::common::interfaces::SettingsStore;
use agent_application::common::models::StatusHooksRequest;
use kernel::AgentId;
use serde_json::{Value, json};
use wire::HOOK_PATH_PREFIX;

use crate::bootstrap::ServerLog;

/// The environment variable that carries the session secret to the hooks of
/// the CLI. The settings file holds no secret (0024 D3).
pub const HOOK_SECRET_VARIABLE: &str = "OFFICE_HARNESS_HOOK_SECRET";

/// The option that names the office settings file.
pub const SETTINGS_OPTION: &str = "--settings";

/// The first version of `claude` known to have `http` hooks (0024 D5).
const MIN_HOOK_VERSION: [u32; 3] = [2, 1, 295];

/// How long `<program> --version` may run.
const VERSION_PROBE_LIMIT: Duration = Duration::from_secs(5);
const VERSION_PROBE_POLL: Duration = Duration::from_millis(25);

/// The longest one report may delay the CLI (0024/FR-006).
const HOOK_TIMEOUT_SECONDS: u32 = 2;

/// The events of the CLI whose reports the server maps to its event kinds.
const HOOK_EVENTS: [&str; 6] = [
    "UserPromptSubmit",
    "Stop",
    "StopFailure",
    "PermissionRequest",
    "PreCompact",
    "PostCompact",
];

const SETTINGS_FILE_EXTENSION: &str = "json";

const NOTE_NO_HOOKS: &str = "this CLI has no status hooks";
const NOTE_SWITCHED_OFF: &str = "hook status is off";
const NOTE_NOT_LISTENING: &str = "the office was not listening yet";
const NOTE_SETTINGS_UNREADABLE: &str = "the office settings could not be read";
const NOTE_OLD_VERSION: &str = "the CLI version does not support status hooks";
const NOTE_FILE_FAILED: &str = "the office settings file could not be written";
const NOTE_FIRST_REPORT: &str = "no report has arrived yet";

/// What `plan` decided for one session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookPlan {
    /// The arguments that name the office settings file, empty for a session
    /// that gets none.
    pub arguments: Vec<String>,
    /// Why the session is estimated, until its first report arrives.
    pub note: String,
}

/// The office settings files of the `claude` sessions and the secrets that
/// go with them (0024 D3, D13). A file and its secret live from the spawn of
/// a session to its end. Clones share one record.
#[derive(Clone)]
pub struct StatusHookFiles {
    inner: Arc<Inner>,
}

struct Inner {
    folder: PathBuf,
    secrets: Arc<dyn HookSecretStore>,
    settings: Arc<dyn SettingsStore>,
    log: ServerLog,
    /// The port of the server's listener, known once it is bound.
    port: OnceLock<u16>,
    /// The version that each program answered, for the server's life.
    versions: Mutex<HashMap<String, Option<[u32; 3]>>>,
    /// The session whose file each running agent has.
    sessions: Mutex<HashMap<AgentId, String>>,
}

impl StatusHookFiles {
    #[must_use]
    pub fn new(
        folder: PathBuf,
        secrets: Arc<dyn HookSecretStore>,
        settings: Arc<dyn SettingsStore>,
        log: ServerLog,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                folder,
                secrets,
                settings,
                log,
                port: OnceLock::new(),
                versions: Mutex::default(),
                sessions: Mutex::default(),
            }),
        }
    }

    /// Files over default settings and a fresh secret store, for the tests
    /// of the bridges that only need something to end.
    #[cfg(test)]
    pub(crate) fn for_tests(folder: PathBuf, log: ServerLog) -> Self {
        struct DefaultSettings;
        impl SettingsStore for DefaultSettings {
            fn load(
                &self,
            ) -> Result<
                agent_domain::value_objects::Settings,
                agent_application::common::errors::PortError,
            > {
                Ok(agent_domain::value_objects::Settings::default())
            }
            fn save(
                &self,
                _settings: &agent_domain::value_objects::Settings,
            ) -> Result<(), agent_application::common::errors::PortError> {
                Ok(())
            }
        }
        Self::new(
            folder,
            Arc::new(access_infrastructure::services::MemoryHookSecretStore::new().expect("store")),
            Arc::new(DefaultSettings),
            log,
        )
    }

    /// Tells the files where the listener is. The first port stays.
    pub fn set_port(&self, port: u16) {
        let _ = self.inner.port.set(port);
    }

    /// Decides whether the session of `session_id` gets the office settings
    /// file, and names its arguments. Nothing is written yet: the host
    /// writes the file right before the spawn, so a start that never runs
    /// leaves nothing behind.
    #[must_use]
    pub fn plan(
        &self,
        request: &StatusHooksRequest,
        session_id: &str,
        program: &str,
        environment: &[(String, String)],
    ) -> HookPlan {
        match self.reason_for_none(request, program, environment) {
            Some(note) => HookPlan {
                arguments: Vec::new(),
                note,
            },
            None => HookPlan {
                arguments: vec![
                    SETTINGS_OPTION.to_owned(),
                    self.file_path(session_id).display().to_string(),
                ],
                note: NOTE_FIRST_REPORT.to_owned(),
            },
        }
    }

    fn reason_for_none(
        &self,
        request: &StatusHooksRequest,
        program: &str,
        environment: &[(String, String)],
    ) -> Option<String> {
        match request {
            StatusHooksRequest::Unsupported => return Some(NOTE_NO_HOOKS.to_owned()),
            StatusHooksRequest::Refused(reason) => return Some(reason.clone()),
            StatusHooksRequest::Wanted => {}
        }
        match self.inner.settings.load() {
            Ok(settings) if !settings.hook_status => return Some(NOTE_SWITCHED_OFF.to_owned()),
            Ok(_) => {}
            Err(error) => {
                self.inner
                    .log
                    .error(&format!("the status hook setting cannot be read: {error}"));
                return Some(NOTE_SETTINGS_UNREADABLE.to_owned());
            }
        }
        if self.inner.port.get().is_none() {
            return Some(NOTE_NOT_LISTENING.to_owned());
        }
        if !self.supports_hooks(program, environment) {
            return Some(NOTE_OLD_VERSION.to_owned());
        }
        None
    }

    /// Writes the file of `session_id` and issues its secret, for the agent.
    /// Returns the environment variable for the spawn, or the reason that
    /// the session runs without hooks.
    ///
    /// # Errors
    /// The note for the agent's details when the secret or the file cannot
    /// be made.
    pub fn activate(
        &self,
        agent_id: AgentId,
        session_id: &str,
    ) -> Result<(String, String), String> {
        let Some(port) = self.inner.port.get().copied() else {
            return Err(NOTE_NOT_LISTENING.to_owned());
        };
        // A new session replaces the files of the agent's earlier one.
        self.end(agent_id);
        let secret = self
            .inner
            .secrets
            .issue(agent_id, session_id)
            .map_err(|error| {
                self.inner
                    .log
                    .error(&format!("a hook secret cannot be made: {error}"));
                NOTE_FILE_FAILED.to_owned()
            })?;
        if let Err(error) = self.write_file(session_id, port) {
            self.inner.secrets.revoke_session(session_id);
            self.inner.log.error(&format!(
                "the office settings file of a session cannot be written: {error}"
            ));
            return Err(NOTE_FILE_FAILED.to_owned());
        }
        self.sessions().insert(agent_id, session_id.to_owned());
        Ok((
            HOOK_SECRET_VARIABLE.to_owned(),
            secret.expose_secret().to_owned(),
        ))
    }

    /// The agent's session ends, or never ran: its secret and its file go.
    pub fn end(&self, agent_id: AgentId) {
        self.inner.secrets.revoke_agent(agent_id);
        let session = self.sessions().remove(&agent_id);
        if let Some(session_id) = session {
            self.remove_file(&self.file_path(&session_id));
        }
    }

    /// The server stops: every session ends, so every secret and file goes
    /// (0024/FR-010).
    pub fn end_all(&self) {
        let agents: Vec<AgentId> = self.sessions().keys().copied().collect();
        for agent_id in agents {
            self.end(agent_id);
        }
        self.sweep_stale();
    }

    /// Deletes every office settings file in the folder. At the server's
    /// start no session runs, so none belongs to one (0024/FR-010).
    pub fn sweep_stale(&self) {
        let Ok(entries) = std::fs::read_dir(&self.inner.folder) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|ext| ext == SETTINGS_FILE_EXTENSION)
            {
                self.remove_file(&path);
            }
        }
    }

    fn sessions(&self) -> MutexGuard<'_, HashMap<AgentId, String>> {
        self.inner
            .sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn file_path(&self, session_id: &str) -> PathBuf {
        self.inner
            .folder
            .join(format!("{session_id}.{SETTINGS_FILE_EXTENSION}"))
    }

    fn write_file(&self, session_id: &str, port: u16) -> std::io::Result<()> {
        private_fs::create_private_dir_all(&self.inner.folder)?;
        let contents = settings_file_text(port, session_id);
        private_fs::replace_private_file(&self.file_path(session_id), contents.as_bytes())
    }

    fn remove_file(&self, path: &Path) {
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => self.inner.log.error(&format!(
                "the office settings file {} cannot be deleted: {error}",
                path.display()
            )),
        }
    }

    fn supports_hooks(&self, program: &str, environment: &[(String, String)]) -> bool {
        let known = self
            .inner
            .versions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(program)
            .copied();
        let version = known.unwrap_or_else(|| {
            let probed = probe_version(program, environment);
            self.inner
                .versions
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(program.to_owned(), probed);
            probed
        });
        version.is_some_and(|version| version >= MIN_HOOK_VERSION)
    }
}

/// The text of the office settings file: one `http` hook per event, with the
/// secret read from the session's environment (0024 D3).
fn settings_file_text(port: u16, session_id: &str) -> String {
    let hook = json!({
        "type": "http",
        "url": format!("http://127.0.0.1:{port}{HOOK_PATH_PREFIX}{session_id}"),
        "headers": {"Authorization": format!("Bearer ${HOOK_SECRET_VARIABLE}")},
        "allowedEnvVars": [HOOK_SECRET_VARIABLE],
        "timeout": HOOK_TIMEOUT_SECONDS,
    });
    let hooks: serde_json::Map<String, Value> = HOOK_EVENTS
        .iter()
        .map(|event| ((*event).to_owned(), json!([{"hooks": [hook.clone()]}])))
        .collect();
    json!({ "hooks": hooks }).to_string()
}

/// Runs `<program> --version` and reads the first `major.minor.patch` of its
/// answer. A program that does not answer in time, or whose answer has no
/// version, has none.
fn probe_version(program: &str, environment: &[(String, String)]) -> Option<[u32; 3]> {
    let mut child = Command::new(program)
        .arg("--version")
        .env_clear()
        .envs(environment.iter().map(|(name, value)| (name, value)))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + VERSION_PROBE_LIMIT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(VERSION_PROBE_POLL),
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let mut text = String::new();
    child.stdout.take()?.read_to_string(&mut text).ok()?;
    parse_version(&text)
}

/// The first `major.minor.patch` in `text`.
fn parse_version(text: &str) -> Option<[u32; 3]> {
    text.split(|character: char| !(character.is_ascii_digit() || character == '.'))
        .find_map(|word| {
            let mut parts = word.split('.').map(str::parse::<u32>);
            let version = [
                parts.next()?.ok()?,
                parts.next()?.ok()?,
                parts.next()?.ok()?,
            ];
            parts.next().is_none().then_some(version)
        })
}

#[cfg(test)]
#[path = "status_hook_files.tests.rs"]
mod tests;

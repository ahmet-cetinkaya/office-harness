// SPDX-License-Identifier: AGPL-3.0-or-later

//! In-memory fakes of the ports, shared by the unit tests.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use agent_domain::entities::{Agent, Project};
use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::{AgentCommand, DisplayName, MachineId, ProjectId, Settings};
use kernel::{AgentId, Timestamp};

use crate::commands::create_agent::CreateAgentHandler;
use crate::commands::delete_agent::DeleteAgentHandler;
use crate::commands::start_agent::StartAgentHandler;
use crate::commands::stop_agent::StopAgentHandler;
use crate::common::action_journal::ActionJournal;
use crate::common::errors::{PortError, TerminalHostError};
use crate::common::interfaces::{
    ActionRecorder, AgentStore, Clock, ProjectStore, SettingsStore, TerminalHost,
};
use crate::common::models::{AgentActionEntry, StartTerminal};

const NOW_MILLIS: i64 = 1_700_000_000_000;

fn locked<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[must_use]
pub fn absolute_folder(name: &str) -> PathBuf {
    std::env::temp_dir().join(name)
}

#[must_use]
pub fn project_named(name: &str) -> Project {
    Project::new(
        ProjectId::generate(),
        DisplayName::parse("project name", name).expect("name"),
        absolute_folder("project"),
    )
    .expect("project")
}

#[must_use]
pub fn agent_in_project(name: &str, project_id: ProjectId) -> Agent {
    Agent::new(
        AgentId::generate(),
        DisplayName::parse("agent name", name).expect("name"),
        AgentCommand::parse("claude").expect("command"),
        None,
        Some(project_id),
        MachineId::generate(),
        Timestamp::from_unix_millis(NOW_MILLIS),
    )
    .expect("agent")
}

#[derive(Default)]
pub struct MemoryAgents(Mutex<Vec<Agent>>);

impl MemoryAgents {
    #[must_use]
    pub fn all(&self) -> Vec<Agent> {
        locked(&self.0).clone()
    }

    #[must_use]
    pub fn status_of(&self, id: AgentId) -> Option<AgentStatus> {
        locked(&self.0)
            .iter()
            .find(|agent| agent.id == id)
            .map(|agent| agent.status)
    }
}

impl AgentStore for MemoryAgents {
    fn insert(&self, agent: &Agent) -> Result<(), PortError> {
        let mut agents = locked(&self.0);
        let key = agent.name.uniqueness_key();
        if agents
            .iter()
            .any(|other| other.name.uniqueness_key() == key)
        {
            return Err(PortError::Duplicate {
                what: format!("agent {}", agent.name),
            });
        }
        agents.push(agent.clone());
        Ok(())
    }

    fn find(&self, id: AgentId) -> Result<Option<Agent>, PortError> {
        Ok(locked(&self.0).iter().find(|agent| agent.id == id).cloned())
    }

    fn list(&self) -> Result<Vec<Agent>, PortError> {
        Ok(self.all())
    }

    fn update_status(&self, id: AgentId, status: AgentStatus) -> Result<(), PortError> {
        let mut agents = locked(&self.0);
        let agent = agents
            .iter_mut()
            .find(|agent| agent.id == id)
            .ok_or_else(|| PortError::NotFound {
                what: format!("agent {id}"),
            })?;
        agent.status = status;
        Ok(())
    }

    fn delete(&self, id: AgentId) -> Result<(), PortError> {
        let mut agents = locked(&self.0);
        let before = agents.len();
        agents.retain(|agent| agent.id != id);
        if agents.len() == before {
            return Err(PortError::NotFound {
                what: format!("agent {id}"),
            });
        }
        Ok(())
    }
}

pub struct MemoryProjects {
    projects: Mutex<Vec<Project>>,
    agents: Arc<MemoryAgents>,
}

impl MemoryProjects {
    #[must_use]
    pub fn new(agents: Arc<MemoryAgents>) -> Self {
        Self {
            projects: Mutex::new(Vec::new()),
            agents,
        }
    }

    #[must_use]
    pub fn all(&self) -> Vec<Project> {
        locked(&self.projects).clone()
    }
}

impl ProjectStore for MemoryProjects {
    fn insert(&self, project: &Project) -> Result<(), PortError> {
        let mut projects = locked(&self.projects);
        let key = project.name.uniqueness_key();
        if projects
            .iter()
            .any(|other| other.name.uniqueness_key() == key)
        {
            return Err(PortError::Duplicate {
                what: format!("project {}", project.name),
            });
        }
        projects.push(project.clone());
        Ok(())
    }

    fn find(&self, id: ProjectId) -> Result<Option<Project>, PortError> {
        Ok(locked(&self.projects)
            .iter()
            .find(|project| project.id == id)
            .cloned())
    }

    fn list(&self) -> Result<Vec<Project>, PortError> {
        Ok(self.all())
    }

    fn delete(&self, id: ProjectId) -> Result<(), PortError> {
        let users: Vec<String> = self
            .agents
            .all()
            .iter()
            .filter(|agent| agent.project_id == Some(id))
            .map(|agent| agent.name.to_string())
            .collect();
        let mut projects = locked(&self.projects);
        let name = projects
            .iter()
            .find(|project| project.id == id)
            .map(|project| project.name.to_string())
            .ok_or_else(|| PortError::NotFound {
                what: format!("project {id}"),
            })?;
        if !users.is_empty() {
            return Err(PortError::ProjectInUse {
                project: name,
                agents: users,
            });
        }
        projects.retain(|project| project.id != id);
        Ok(())
    }
}

#[derive(Default)]
pub struct MemorySettings(Mutex<Settings>);

impl MemorySettings {
    #[must_use]
    pub fn current(&self) -> Settings {
        locked(&self.0).clone()
    }
}

impl SettingsStore for MemorySettings {
    fn load(&self) -> Result<Settings, PortError> {
        Ok(self.current())
    }

    fn save(&self, settings: &Settings) -> Result<(), PortError> {
        locked(&self.0).clone_from(settings);
        Ok(())
    }
}

struct FixedClock;

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_unix_millis(NOW_MILLIS)
    }
}

#[derive(Default)]
pub struct RecordingRecorder(Mutex<Vec<AgentActionEntry>>);

impl RecordingRecorder {
    #[must_use]
    pub fn entries(&self) -> Vec<AgentActionEntry> {
        locked(&self.0).clone()
    }
}

impl ActionRecorder for RecordingRecorder {
    fn record(&self, entry: &AgentActionEntry) -> Result<(), PortError> {
        locked(&self.0).push(entry.clone());
        Ok(())
    }
}

/// A terminal host that starts nothing: it keeps the requests, and a command
/// whose program is in `unstartable` fails with a text.
#[derive(Default)]
pub struct FakeTerminals {
    started: Mutex<Vec<StartTerminal>>,
    stopped: Mutex<Vec<(AgentId, Duration)>>,
    unstartable: Mutex<Vec<String>>,
    ended_on_its_own: Mutex<bool>,
}

impl FakeTerminals {
    #[must_use]
    pub fn started(&self) -> Vec<StartTerminal> {
        locked(&self.started).clone()
    }

    #[must_use]
    pub fn stopped(&self) -> Vec<(AgentId, Duration)> {
        locked(&self.stopped).clone()
    }

    /// From now on a stop finds no running terminal, as when the child
    /// exited just before the request.
    pub fn end_children_on_their_own(&self) {
        *locked(&self.ended_on_its_own) = true;
    }

    pub fn make_unstartable(&self, program: &str) {
        locked(&self.unstartable).push(program.to_owned());
    }
}

impl TerminalHost for FakeTerminals {
    fn start(&self, request: &StartTerminal) -> Result<(), TerminalHostError> {
        if locked(&self.unstartable).contains(&request.program) {
            return Err(TerminalHostError::Failed(format!(
                "the command {} was not found on the login PATH",
                request.program
            )));
        }
        locked(&self.started).push(request.clone());
        Ok(())
    }

    fn stop(&self, agent_id: AgentId, grace: Duration) -> Result<(), TerminalHostError> {
        if *locked(&self.ended_on_its_own) {
            return Err(TerminalHostError::NotRunning);
        }
        locked(&self.stopped).push((agent_id, grace));
        Ok(())
    }
}

/// Every port as an in-memory fake, with the handlers built on them.
pub struct Office {
    pub agents: Arc<MemoryAgents>,
    pub projects: Arc<MemoryProjects>,
    pub settings: Arc<MemorySettings>,
    pub recorder: Arc<RecordingRecorder>,
    pub terminals: Arc<FakeTerminals>,
    pub machine_id: MachineId,
}

impl Office {
    #[must_use]
    pub fn new() -> Self {
        let agents = Arc::new(MemoryAgents::default());
        Self {
            projects: Arc::new(MemoryProjects::new(agents.clone())),
            agents,
            settings: Arc::new(MemorySettings::default()),
            recorder: Arc::new(RecordingRecorder::default()),
            terminals: Arc::new(FakeTerminals::default()),
            machine_id: MachineId::generate(),
        }
    }

    fn journal(&self) -> ActionJournal {
        ActionJournal::new(self.recorder.clone(), Arc::new(FixedClock))
    }

    #[must_use]
    pub fn create_agent(&self) -> CreateAgentHandler {
        CreateAgentHandler::new(
            self.agents.clone(),
            self.projects.clone(),
            Arc::new(FixedClock),
            self.journal(),
            self.machine_id,
        )
    }

    #[must_use]
    pub fn start_agent(&self) -> StartAgentHandler {
        StartAgentHandler::new(
            self.agents.clone(),
            self.projects.clone(),
            self.terminals.clone(),
            self.journal(),
        )
    }

    #[must_use]
    pub fn stop_agent(&self) -> StopAgentHandler {
        StopAgentHandler::new(
            self.agents.clone(),
            self.settings.clone(),
            self.terminals.clone(),
            self.journal(),
        )
    }

    #[must_use]
    pub fn delete_agent(&self) -> DeleteAgentHandler {
        DeleteAgentHandler::new(
            self.agents.clone(),
            self.settings.clone(),
            self.terminals.clone(),
            self.journal(),
        )
    }

    /// Stores an agent with the command, folder, and status given.
    pub fn add_agent(
        &self,
        name: &str,
        command: &str,
        folder: &Path,
        status: AgentStatus,
    ) -> Agent {
        let mut agent = Agent::new(
            AgentId::generate(),
            DisplayName::parse("agent name", name).expect("name"),
            AgentCommand::parse(command).expect("command"),
            Some(folder.to_path_buf()),
            None,
            self.machine_id,
            Timestamp::from_unix_millis(NOW_MILLIS),
        )
        .expect("agent");
        agent.status = status;
        self.agents.insert(&agent).expect("insert");
        agent
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashSet;
use std::path::PathBuf;

use agent_application::common::interfaces::ProjectStore;
use agent_domain::entities::Project;
use hierarchy_application::common::ports::{AgentCreator, AgentSeed, SeedAdditions};
use kernel::ProjectId;
use template_domain::{AdditionsBody, AgentBody};

const NO_FOLDER_REASON: &str =
    "a working folder is needed: the template has none and no project is given";

/// The pre-check of one creation (D5 "Plan"). It resolves the name, the
/// working folder, and the project of each agent, asks the agent module's
/// own rules about each seed, and keeps every reason instead of stopping at
/// the first one (FR-005). Nothing is written.
pub(super) struct Planner<'a> {
    creator: &'a dyn AgentCreator,
    /// Lower-cased names of the office's agents and of the agents planned so
    /// far in this creation, so no two get the same name (D4).
    taken_names: HashSet<String>,
    project_id: Option<ProjectId>,
    project_requested: bool,
    project_folder: Option<PathBuf>,
    reasons: Vec<String>,
}

impl<'a> Planner<'a> {
    pub(super) fn new(
        creator: &'a dyn AgentCreator,
        existing_names: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            creator,
            taken_names: existing_names
                .into_iter()
                .map(|name| name.to_lowercase())
                .collect(),
            project_id: None,
            project_requested: false,
            project_folder: None,
            reasons: Vec::new(),
        }
    }

    /// Fixes the project of every planned agent. An unknown project is a
    /// reason, not a stop, so the other reasons still show.
    pub(super) fn use_project(
        &mut self,
        projects: &dyn ProjectStore,
        project_id: Option<ProjectId>,
    ) {
        self.project_requested = project_id.is_some();
        let Some(project_id) = project_id else {
            return;
        };
        match projects.find(project_id) {
            Ok(Some(Project { folder, .. })) => {
                self.project_id = Some(project_id);
                self.project_folder = Some(folder);
            }
            Ok(None) => self.add_reason(format!("project {project_id} was not found")),
            Err(error) => self.add_reason(error.to_string()),
        }
    }

    /// The project that the planned agents get: the requested one, when it
    /// exists.
    pub(super) fn effective_project_id(&self) -> Option<ProjectId> {
        self.project_id
    }

    pub(super) fn add_reason(&mut self, reason: String) {
        self.reasons.push(reason);
    }

    /// The seed of one agent made from `body`, or `None` when the name
    /// cannot be made. `team_name` fills `{team}`.
    pub(super) fn plan_agent(
        &mut self,
        body: &AgentBody,
        team_name: Option<&str>,
    ) -> Option<AgentSeed> {
        let name = self.unique_name(body, team_name)?;
        let seed = AgentSeed {
            name,
            command: body.command.clone(),
            working_folder: body
                .working_folder
                .as_deref()
                .filter(|folder| !folder.is_empty())
                .map(PathBuf::from),
            project_id: self.project_id,
            additions: Some(seed_additions(&body.additions)),
            // A template holds no own worktree choice (0009 scope).
            worktree: None,
        };
        for reason in self.creator.check(&seed) {
            self.add_reason(format!("{}: {reason}", seed.name));
        }
        self.check_folder(&seed);
        Some(seed)
    }

    /// Ends the plan: `Ok` when no reason was found, else each reason once,
    /// in the order found.
    pub(super) fn finish(self) -> Result<(), Vec<String>> {
        let mut seen = HashSet::new();
        let reasons: Vec<String> = self
            .reasons
            .into_iter()
            .filter(|reason| seen.insert(reason.clone()))
            .collect();
        if reasons.is_empty() {
            Ok(())
        } else {
            Err(reasons)
        }
    }

    fn unique_name(&mut self, body: &AgentBody, team_name: Option<&str>) -> Option<String> {
        let name = body.name_pattern.unique_name(team_name, |candidate| {
            self.taken_names.contains(&candidate.to_lowercase())
        });
        let Some(name) = name else {
            self.add_reason(format!(
                "the name pattern \"{}\" needs a team, and this creation has none",
                body.name_pattern.as_str()
            ));
            return None;
        };
        self.taken_names.insert(name.to_lowercase());
        Some(name)
    }

    /// The folder the agent will start in must exist now, so a typo does not
    /// show only at its first start (FR-005).
    fn check_folder(&mut self, seed: &AgentSeed) {
        let folder = seed
            .working_folder
            .as_deref()
            .or(self.project_folder.as_deref());
        match folder {
            Some(folder) if !folder.is_dir() => self.add_reason(format!(
                "the working folder \"{}\" is not a folder that exists",
                folder.display()
            )),
            None if !self.project_requested => self.add_reason(NO_FOLDER_REASON.to_owned()),
            _ => {}
        }
    }
}

fn seed_additions(additions: &AdditionsBody) -> SeedAdditions {
    SeedAdditions {
        flags: additions.flags.clone(),
        env: additions
            .env
            .iter()
            .map(|variable| (variable.name.clone(), variable.value.clone()))
            .collect(),
        startup_prompt: additions.startup_prompt.clone(),
    }
}

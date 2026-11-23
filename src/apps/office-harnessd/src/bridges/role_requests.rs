// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_application::commands::create_agent::OwnWorktree;
use agent_application::commands::delete_agent::DeleteWorktrees;
use agent_application::common::additions_input::AdditionsInput;
use agent_application::common::command_line::split_command;
use agent_domain::value_objects::{AgentCommand, DisplayName};
use agent_presentation::{RoleRequests, RoleResult, RoleSeed};
use hierarchy_application::commands::create_role_agent::{
    CreateRoleAgentHandler, CreateRoleAgentRequest,
};
use hierarchy_application::common::errors::HierarchyApplicationError;
use hierarchy_application::common::interfaces::HierarchyStore;
use hierarchy_application::common::ports::{
    AgentCreator, AgentSeed, CreatorError, CreatorErrorKind, SeedAdditions, SeedWorktree,
};
use hierarchy_domain::{Role, TeamId};
use kernel::AgentId;
use wire::ErrorCode;

/// Answers the agent module's role port with the hierarchy's one-step
/// create-and-place (0002, FR-003). The bridge holds the hierarchy store's
/// tree exclusion across the check and the create+place, so two racing
/// creates see one tree (FR-006, FR-007).
pub struct RoleRequestsBridge {
    create_role_agent: CreateRoleAgentHandler,
    store: Arc<dyn HierarchyStore>,
}

impl RoleRequestsBridge {
    #[must_use]
    pub fn new(create_role_agent: CreateRoleAgentHandler, store: Arc<dyn HierarchyStore>) -> Self {
        Self {
            create_role_agent,
            store,
        }
    }
}

impl RoleRequests for RoleRequestsBridge {
    fn create_role_agent(
        &self,
        seed: &RoleSeed,
        kind: &str,
        parent_agent_id: Option<String>,
        team_id: Option<String>,
    ) -> RoleResult {
        let request = seed_and_place(seed, kind, parent_agent_id, team_id);
        let request = match request {
            Ok(request) => request,
            Err(message) => return RoleResult::failed(ErrorCode::Invalid, message),
        };
        // The exclusion is the hierarchy store's own, so this create is
        // ordered against every other tree writer, the user's creates
        // included (FR-006, FR-007).
        let tree = self.store.tree_exclusion();
        match self.create_role_agent.handle(request, &tree) {
            Ok(agent_id) => RoleResult::Created(agent_id.to_string()),
            Err(error) => RoleResult::failed(wire_code(&error), error.to_string()),
        }
    }
}

/// The role kinds the contract names, with the place they carry: a CEO
/// takes neither parent nor team, a lead a team, a member a lead. Anything
/// else names the field at fault (the closed allowlist of the contract).
/// The request the bridge hands the handler: the place and the seed as the
/// client sent them, every broken field named.
fn seed_and_place(
    seed: &RoleSeed,
    kind: &str,
    parent_agent_id: Option<String>,
    team_id: Option<String>,
) -> Result<CreateRoleAgentRequest, String> {
    let (role, parent, team) = place_of(kind, parent_agent_id, team_id)?;
    Ok(CreateRoleAgentRequest {
        seed: seed_of(seed)?,
        role,
        parent_agent_id: parent,
        team_id: team,
    })
}

fn place_of(
    kind: &str,
    parent_agent_id: Option<String>,
    team_id: Option<String>,
) -> Result<(Role, Option<AgentId>, Option<TeamId>), String> {
    let role = Role::from_wire(kind)
        .ok_or_else(|| format!("role.kind is not a role of the company: {kind}"))?;
    let parent = parent_agent_id
        .map(|text| {
            AgentId::from_uuid_text(&text)
                .ok_or_else(|| "role.parent_agent_id is not an agent id".to_owned())
        })
        .transpose()?;
    let team = team_id
        .map(|text| {
            TeamId::from_uuid_text(&text).ok_or_else(|| "role.team_id is not a team id".to_owned())
        })
        .transpose()?;
    Ok((role, parent, team))
}

/// The seed as the hierarchy's port carries it: the same fields, the
/// additions mirrored (the hierarchy crate cannot name the agent module's
/// type, so the bridge copies the three fields).
fn seed_of(seed: &RoleSeed) -> Result<AgentSeed, String> {
    let project_id = seed
        .project_id
        .as_deref()
        .map(|text| {
            kernel::ProjectId::from_uuid_text(text)
                .ok_or_else(|| format!("project_id is not a valid identifier: {text}"))
        })
        .transpose()?;
    Ok(AgentSeed {
        temp: None,
        name: seed.name.clone(),
        command: seed.command.clone(),
        working_folder: seed
            .working_folder
            .as_deref()
            .filter(|folder| !folder.is_empty())
            .map(std::path::PathBuf::from),
        project_id,
        additions: seed.additions.as_ref().map(|additions| SeedAdditions {
            flags: additions.flags.clone(),
            env: additions.env.clone(),
            startup_prompt: additions.startup_prompt.clone(),
        }),
        worktree: seed.worktree.as_ref().map(|own| SeedWorktree {
            base: own.base.clone(),
            branch_name: own.branch_name.clone(),
        }),
    })
}

/// The wire code of a hierarchy failure, as the hierarchy handlers map it.
fn wire_code(error: &HierarchyApplicationError) -> ErrorCode {
    match error {
        HierarchyApplicationError::Invalid(_) | HierarchyApplicationError::InvalidInput(_) => {
            ErrorCode::Invalid
        }
        // The `*Input` variants carry an agent-module message, so they keep
        // the code `agent.create` answers with — only the prefix is bare.
        HierarchyApplicationError::NotFoundInput(_) | HierarchyApplicationError::NotFound(_) => {
            ErrorCode::NotFound
        }
        HierarchyApplicationError::ConflictInput(_) | HierarchyApplicationError::Conflict(_) => {
            ErrorCode::Conflict
        }
        // A hold or a rate limit refused the action (0007).
        HierarchyApplicationError::Refused(_) => ErrorCode::Refused,
        HierarchyApplicationError::Storage(_) => ErrorCode::Internal,
    }
}

/// Creates one agent through the agent module's own create use case, so a
/// seeded create applies the same rules as `agent.create` (the name and
/// command rules, the additions rules, the journal entry), and removes it
/// again through the delete use case when the tree cannot place it.
pub struct AgentCreatorBridge {
    create: agent_application::commands::create_agent::CreateAgentHandler,
    delete: agent_application::commands::delete_agent::DeleteAgentHandler,
}

impl AgentCreatorBridge {
    #[must_use]
    pub fn new(
        create: agent_application::commands::create_agent::CreateAgentHandler,
        delete: agent_application::commands::delete_agent::DeleteAgentHandler,
    ) -> Self {
        Self { create, delete }
    }
}

impl AgentCreator for AgentCreatorBridge {
    fn check(&self, seed: &AgentSeed) -> Vec<String> {
        agent_rule_failures(seed)
    }

    fn create(&self, seed: AgentSeed) -> Result<AgentId, CreatorError> {
        let agent = self
            .create
            .handle(
                agent_application::commands::create_agent::CreateAgentRequest {
                    name: seed.name,
                    command: seed.command,
                    working_folder: seed.working_folder,
                    project_id: seed.project_id,
                    additions: seed.additions.map(additions_input),
                    worktree: seed.worktree.map(|own| OwnWorktree {
                        base: own.base,
                        branch_name: own.branch_name,
                    }),
                    temp: None,
                },
            )
            .map_err(|error| agent_failure(&error))?;
        Ok(agent.id)
    }

    /// The create undo: the worktree and branch the create made go too,
    /// without asking (0010 D5).
    fn remove(&self, agent_id: AgentId) -> Result<(), CreatorError> {
        self.delete
            .handle(agent_id, &DeleteWorktrees::Discard)
            .map(|_| ())
            .map_err(|error| agent_failure(&error))
    }
}

fn additions_input(additions: SeedAdditions) -> AdditionsInput {
    AdditionsInput {
        flags: additions.flags,
        env: additions.env,
        startup_prompt: additions.startup_prompt,
    }
}

/// Every create rule of the agent module that `seed` breaks, found with the
/// module's own value objects and checks, so a pre-check never drifts from
/// the create. The rules are independent, so each one reports on its own.
fn agent_rule_failures(seed: &AgentSeed) -> Vec<String> {
    let mut reasons = Vec::new();
    if let Err(error) = DisplayName::parse("agent name", &seed.name) {
        reasons.push(error.to_string());
    }
    match AgentCommand::parse(&seed.command) {
        Ok(command) => {
            if let Err(error) = split_command(&command) {
                reasons.push(error.to_string());
            }
        }
        Err(error) => reasons.push(error.to_string()),
    }
    let additions = additions_input(seed.additions.clone().unwrap_or_default());
    if let Err(error) = additions.into_additions() {
        reasons.push(error.to_string());
    }
    reasons
}

/// The agent module's refusal as the hierarchy's port carries it: the kind
/// is the application error's own, so the wire code stays the one
/// `agent.create` answers with.
fn agent_failure(error: &agent_application::common::errors::ApplicationError) -> CreatorError {
    CreatorError {
        kind: match error {
            agent_application::common::errors::ApplicationError::Invalid(_) => {
                CreatorErrorKind::Invalid
            }
            agent_application::common::errors::ApplicationError::NotFound(_) => {
                CreatorErrorKind::NotFound
            }
            agent_application::common::errors::ApplicationError::Conflict(_) => {
                CreatorErrorKind::Conflict
            }
            _ => CreatorErrorKind::Internal,
        },
        message: error.to_string(),
    }
}

#[cfg(test)]
#[path = "role_requests.tests.rs"]
mod tests;

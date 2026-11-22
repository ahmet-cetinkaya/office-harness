// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_application::common::interfaces::{AgentStore, ProjectStore};
use agent_domain::entities::Agent;
use hierarchy_application::common::interfaces::{HierarchyStore, TreeExclusion, TreeSnapshot};
use hierarchy_application::common::ports::{AgentCreator, AgentSeed};
use hierarchy_domain::{
    Placement, Role, Team, TeamId, TeamName, TreeRow, check_placement, resolved_parent,
};
use kernel::{AgentId, ProjectId};
use template_application::common::interfaces::TemplateStore;
use template_domain::{AgentBody, Template, TemplateBody, TemplateId, TemplateRole};

use crate::bootstrap::StorageAlerts;

use super::template_create_error::TemplateCreateError;
use super::template_plan::Planner;

/// Where a direct creation puts an agent whose template has a role (D4): the
/// parent and the team. A template with no role, and a CEO, take no place.
#[derive(Debug, Clone, Copy)]
pub struct DirectPlace {
    pub parent_agent_id: AgentId,
    pub team_id: TeamId,
}

/// One agent that a creation made, with the place it was given. `agent` is
/// what the store read back after the writes; it is `None` when that read
/// failed, because the agent exists all the same and the creation succeeded.
#[derive(Debug, Clone)]
pub struct CreatedAgent {
    pub id: AgentId,
    pub name: String,
    pub agent: Option<Agent>,
    pub role: Option<Role>,
    pub team_name: Option<String>,
}

/// A team that a creation made, and its agents, the lead first.
#[derive(Debug, Clone)]
pub struct TeamCreation {
    pub team: Team,
    pub agents: Vec<CreatedAgent>,
}

struct PlannedPlace {
    role: Role,
    parent: Option<AgentId>,
    team: Option<TeamId>,
}

struct DirectPlan {
    seed: AgentSeed,
    place: Option<PlannedPlace>,
    team_name: Option<String>,
}

struct TeamPlan {
    team_name: TeamName,
    project_id: Option<ProjectId>,
    parent: AgentId,
    lead: AgentSeed,
    members: Vec<AgentSeed>,
}

/// What a creation has made so far, so a failure can undo exactly that.
#[derive(Default)]
struct Created {
    agents: Vec<CreatedEntry>,
    team: Option<TeamId>,
}

struct CreatedEntry {
    id: AgentId,
    name: String,
}

/// Creates an agent, or a whole team, from a template (0009 D5). It holds the
/// tree exclusion for the whole creation, checks every agent, folder, and
/// place before it creates anything, and undoes what it made when a late step
/// fails, so a creation makes all of its agents or none of them (SC-005).
pub struct TemplateCreateBridge {
    templates: Arc<dyn TemplateStore>,
    store: Arc<dyn HierarchyStore>,
    creator: Arc<dyn AgentCreator>,
    agents: Arc<dyn AgentStore>,
    projects: Arc<dyn ProjectStore>,
    alerts: StorageAlerts,
}

impl TemplateCreateBridge {
    #[must_use]
    pub fn new(
        templates: Arc<dyn TemplateStore>,
        store: Arc<dyn HierarchyStore>,
        creator: Arc<dyn AgentCreator>,
        agents: Arc<dyn AgentStore>,
        projects: Arc<dyn ProjectStore>,
        alerts: StorageAlerts,
    ) -> Self {
        Self {
            templates,
            store,
            creator,
            agents,
            projects,
            alerts,
        }
    }

    /// Creates one agent from an agent template (FR-002).
    ///
    /// # Errors
    /// `invalid` with every reason, `refused` with the tree rule's text,
    /// `not_found`, or `internal`.
    pub fn create_agent(
        &self,
        template_id: TemplateId,
        place: Option<DirectPlace>,
        project_id: Option<ProjectId>,
    ) -> Result<CreatedAgent, TemplateCreateError> {
        let body = self.agent_body(template_id)?;
        let tree = self.store.tree_exclusion();
        let snapshot = self.snapshot()?;
        let plan = self.plan_direct(&body, place, project_id, &snapshot)?;
        let agent_id = self.apply_direct(&tree, &plan)?;
        Ok(self.created_agent(
            CreatedEntry {
                id: agent_id,
                name: plan.seed.name,
            },
            plan.place.map(|place| place.role),
            plan.team_name,
        ))
    }

    /// Creates a team, its lead under `parent_agent_id`, and its members, from
    /// a team template, all or nothing (FR-004, SC-005).
    ///
    /// # Errors
    /// As `create_agent`, and `conflict` for a team name that is taken. A
    /// failed undo names each agent left behind.
    pub fn create_team(
        &self,
        template_id: TemplateId,
        team_name: &str,
        parent_agent_id: AgentId,
        project_id: Option<ProjectId>,
    ) -> Result<TeamCreation, TemplateCreateError> {
        let body = match self.template(template_id)?.body {
            TemplateBody::Team(team) => team,
            TemplateBody::Agent(_) => {
                return Err(wrong_kind(template_id, "an agent", "create_agent"));
            }
        };
        let tree = self.store.tree_exclusion();
        let snapshot = self.snapshot()?;
        let request = TeamRequest {
            team_name,
            parent: parent_agent_id,
            project_id,
        };
        let plan = self.plan_team(&body, &request, &snapshot)?;
        let (team, created) = self.apply_team(&tree, &plan)?;
        let agents = self.created_team_agents(&team, created);
        Ok(TeamCreation { team, agents })
    }

    fn template(&self, id: TemplateId) -> Result<Template, TemplateCreateError> {
        self.templates
            .find(id)
            .map_err(|error| TemplateCreateError::internal(error.to_string()))?
            .ok_or_else(|| TemplateCreateError::not_found(format!("template {id} does not exist")))
    }

    fn agent_body(&self, id: TemplateId) -> Result<AgentBody, TemplateCreateError> {
        match self.template(id)?.body {
            TemplateBody::Agent(agent) => Ok(agent),
            TemplateBody::Team(_) => Err(wrong_kind(id, "a team", "create_team")),
        }
    }

    fn snapshot(&self) -> Result<TreeSnapshot, TemplateCreateError> {
        self.store
            .snapshot()
            .map_err(|error| TemplateCreateError::from_hierarchy(&error))
    }

    fn planner(&self) -> Result<Planner<'_>, TemplateCreateError> {
        let names = self
            .agents
            .list()
            .map_err(|error| TemplateCreateError::internal(error.to_string()))?
            .into_iter()
            .map(|agent| agent.name.as_str().to_owned());
        Ok(Planner::new(self.creator.as_ref(), names))
    }

    fn plan_direct(
        &self,
        body: &AgentBody,
        place: Option<DirectPlace>,
        project_id: Option<ProjectId>,
        snapshot: &TreeSnapshot,
    ) -> Result<DirectPlan, TemplateCreateError> {
        let mut planner = self.planner()?;
        let role = body.role.map(hierarchy_role);
        let team = place.and_then(|place| {
            snapshot
                .teams
                .iter()
                .find(|team| team.id() == place.team_id)
        });
        if let Some(reason) = place_reason(role, place, team) {
            planner.add_reason(reason);
        }
        let project_id = project_of_place(&mut planner, team, project_id);
        planner.use_project(self.projects.as_ref(), project_id);
        let team_name = team.map(|team| team.name().display().to_owned());
        let seed = planner.plan_agent(body, team_name.as_deref());
        planner
            .finish()
            .map_err(|reasons| TemplateCreateError::invalid(&reasons))?;
        let seed = seed.ok_or_else(no_agent_planned)?;
        let place = role
            .map(|role| planned_place(role, place, snapshot))
            .transpose()?;
        Ok(DirectPlan {
            seed,
            place,
            team_name,
        })
    }

    fn plan_team(
        &self,
        body: &template_domain::TeamBody,
        request: &TeamRequest<'_>,
        snapshot: &TreeSnapshot,
    ) -> Result<TeamPlan, TemplateCreateError> {
        let mut planner = self.planner()?;
        let parsed_name = TeamName::parse(request.team_name);
        if let Err(error) = &parsed_name {
            planner.add_reason(error.to_string());
        }
        planner.use_project(self.projects.as_ref(), request.project_id);
        // A bad team name still fills `{team}`, so its own reason is the only
        // one that it causes.
        let label = parsed_name.as_ref().map_or_else(
            |_| request.team_name.trim().to_owned(),
            |name| name.display().to_owned(),
        );
        let lead = planner.plan_agent(&body.lead, Some(&label));
        let members: Vec<Option<AgentSeed>> = body
            .members
            .iter()
            .flat_map(|member| (0..member.count).map(move |_| &member.entry))
            .map(|entry| planner.plan_agent(entry, Some(&label)))
            .collect();
        let project_id = planner.effective_project_id();
        let taken_reason = self.team_name_taken_reason(&parsed_name)?;
        if let Some(reason) = &taken_reason {
            planner.add_reason(reason.clone());
        }
        planner
            .finish()
            .map_err(|reasons| team_plan_failure(&reasons, taken_reason.as_deref()))?;
        let (Ok(team_name), Some(lead), Some(members)) = (
            parsed_name,
            lead,
            members.into_iter().collect::<Option<Vec<_>>>(),
        ) else {
            return Err(no_agent_planned());
        };
        check_team_places(snapshot, request.parent, &lead, &members)?;
        Ok(TeamPlan {
            team_name,
            project_id,
            parent: request.parent,
            lead,
            members,
        })
    }

    /// The reason of a team name that is taken, when it is. It joins the
    /// other reasons, so one answer names everything to change (FR-005).
    fn team_name_taken_reason(
        &self,
        name: &Result<TeamName, hierarchy_domain::HierarchyError>,
    ) -> Result<Option<String>, TemplateCreateError> {
        let Ok(name) = name else {
            return Ok(None);
        };
        let taken = self
            .store
            .team_name_taken(name.key())
            .map_err(|error| TemplateCreateError::from_hierarchy(&error))?;
        Ok(taken.then(|| format!("team name \"{}\" is already used", name.display())))
    }

    fn apply_direct(
        &self,
        tree: &TreeExclusion<'_>,
        plan: &DirectPlan,
    ) -> Result<AgentId, TemplateCreateError> {
        let mut created = Created::default();
        let agent_id = self.create_tracked(&plan.seed, &mut created)?;
        let Some(place) = &plan.place else {
            return Ok(agent_id);
        };
        self.store
            .place_agent(agent_id, Some(place.role), place.parent, place.team)
            .map_err(|error| {
                let failure = TemplateCreateError::from_hierarchy(&error);
                self.undo(tree, &created, failure)
            })?;
        Ok(agent_id)
    }

    fn apply_team(
        &self,
        tree: &TreeExclusion<'_>,
        plan: &TeamPlan,
    ) -> Result<(Team, Created), TemplateCreateError> {
        let mut created = Created::default();
        match self.write_team(tree, plan, &mut created) {
            Ok(team) => Ok((team, created)),
            Err(failure) => Err(self.undo(tree, &created, failure)),
        }
    }

    /// The lead first, then the team row with the lead's place, then each
    /// member. `created` is kept up to date after every write, so the undo
    /// knows what exists at the step that failed.
    fn write_team(
        &self,
        tree: &TreeExclusion<'_>,
        plan: &TeamPlan,
        created: &mut Created,
    ) -> Result<Team, TemplateCreateError> {
        let lead_id = self.create_tracked(&plan.lead, created)?;
        let team = Team::new(plan.team_name.clone(), plan.project_id, lead_id);
        // Recorded before the write: deleting a team row that was never
        // written is harmless, and a half-written one is then still removed.
        created.team = Some(team.id());
        self.store
            .save_team_and_place_lead_held(tree, &team, Some(plan.parent))
            .map_err(|error| TemplateCreateError::from_hierarchy(&error))?;
        for seed in &plan.members {
            let member_id = self.create_tracked(seed, created)?;
            self.store
                .place_agent(
                    member_id,
                    Some(Role::Member),
                    Some(lead_id),
                    Some(team.id()),
                )
                .map_err(|error| TemplateCreateError::from_hierarchy(&error))?;
        }
        Ok(team)
    }

    fn create_tracked(
        &self,
        seed: &AgentSeed,
        created: &mut Created,
    ) -> Result<AgentId, TemplateCreateError> {
        let id = self
            .creator
            .create(seed.clone())
            .map_err(TemplateCreateError::from_creator)?;
        created.agents.push(CreatedEntry {
            id,
            name: seed.name.clone(),
        });
        Ok(id)
    }

    /// Removes what `created` holds, the last agent first, then the team row.
    /// A removal that fails does not stop the rest. The answer is the reason
    /// of the failure that began the undo, plus each leftover, named.
    fn undo(
        &self,
        tree: &TreeExclusion<'_>,
        created: &Created,
        failure: TemplateCreateError,
    ) -> TemplateCreateError {
        let mut leftovers = Vec::new();
        for entry in created.agents.iter().rev() {
            if let Err(error) = self.creator.remove(entry.id) {
                leftovers.push(format!(
                    "agent {} ({}) was left behind: {}",
                    entry.name, entry.id, error.message
                ));
            }
        }
        if let Some(team_id) = created.team
            && let Err(error) = self.store.delete_team_and_unplace_held(tree, team_id)
        {
            leftovers.push(format!(
                "team {} was left behind: {error}",
                team_id.as_uuid_text()
            ));
        }
        if !leftovers.is_empty() {
            self.alerts.report_disk_failure(&leftovers.join("; "));
        }
        failure.with_leftovers(&leftovers)
    }

    /// The creation succeeded, so a read-back that fails must not undo that
    /// answer: the agent is named from the plan, and the failure is raised as
    /// a storage alert.
    fn created_agent(
        &self,
        entry: CreatedEntry,
        role: Option<Role>,
        team_name: Option<String>,
    ) -> CreatedAgent {
        let agent = self.read_back(entry.id);
        CreatedAgent {
            id: entry.id,
            name: entry.name,
            agent,
            role,
            team_name,
        }
    }

    fn read_back(&self, agent_id: AgentId) -> Option<Agent> {
        match self.agents.find(agent_id) {
            Ok(agent) => {
                if agent.is_none() {
                    self.alerts.report_disk_failure(&format!(
                        "agent {agent_id} was not found after its create"
                    ));
                }
                agent
            }
            Err(error) => {
                self.alerts.report_disk_failure(&error.to_string());
                None
            }
        }
    }

    fn created_team_agents(&self, team: &Team, created: Created) -> Vec<CreatedAgent> {
        let team_name = team.name().display();
        created
            .agents
            .into_iter()
            .enumerate()
            .map(|(position, entry)| {
                let role = if position == 0 {
                    Role::Lead
                } else {
                    Role::Member
                };
                self.created_agent(entry, Some(role), Some(team_name.to_owned()))
            })
            .collect()
    }
}

/// What a team creation asks for, besides the template.
struct TeamRequest<'a> {
    team_name: &'a str,
    parent: AgentId,
    project_id: Option<ProjectId>,
}

/// A taken team name alone is a `conflict`; with other reasons it is one of
/// the lines of an `invalid` answer.
fn team_plan_failure(reasons: &[String], taken_reason: Option<&str>) -> TemplateCreateError {
    match (reasons, taken_reason) {
        ([only], Some(taken)) if only == taken => TemplateCreateError::conflict(taken.to_owned()),
        _ => TemplateCreateError::invalid(reasons),
    }
}

fn wrong_kind(id: TemplateId, found: &str, instead: &str) -> TemplateCreateError {
    TemplateCreateError::invalid(&[format!(
        "template {id} is {found} template; use template.{instead} for it"
    )])
}

fn no_agent_planned() -> TemplateCreateError {
    TemplateCreateError::internal("the plan holds no agent although no reason was found".to_owned())
}

fn hierarchy_role(role: TemplateRole) -> Role {
    match role {
        TemplateRole::Ceo => Role::Ceo,
        TemplateRole::Lead => Role::Lead,
        TemplateRole::Member => Role::Member,
    }
}

/// Why the place does not fit the template's role, if it does not (D4): a
/// template with no role and a CEO take no place; a lead and a member need
/// one, with a team that exists.
fn place_reason(
    role: Option<Role>,
    place: Option<DirectPlace>,
    team: Option<&Team>,
) -> Option<String> {
    match (role, place) {
        (None, Some(_)) => Some("place must be absent: the template has no role".to_owned()),
        (Some(Role::Ceo), Some(_)) => Some("place must be absent for a CEO".to_owned()),
        (Some(role @ (Role::Lead | Role::Member)), None) => Some(format!(
            "place is required for a {} template",
            role.as_str()
        )),
        (Some(_), Some(place)) if team.is_none() => Some(format!(
            "team {} does not exist",
            place.team_id.as_uuid_text()
        )),
        _ => None,
    }
}

/// A lead's or member's project is its team's project (0002 A6); a project
/// that the request names and the team does not have is a reason.
fn project_of_place(
    planner: &mut Planner<'_>,
    team: Option<&Team>,
    requested: Option<ProjectId>,
) -> Option<ProjectId> {
    let team_project = team.and_then(Team::project_id);
    if let (Some(team_project), Some(requested)) = (team_project, requested)
        && team_project != requested
    {
        planner.add_reason(
            "a member's or lead's project is its team's project; send no project_id or the team's"
                .to_owned(),
        );
    }
    team_project.or(requested)
}

fn planned_place(
    role: Role,
    place: Option<DirectPlace>,
    snapshot: &TreeSnapshot,
) -> Result<PlannedPlace, TemplateCreateError> {
    // The agent does not exist yet and cannot collide with a row, so the
    // check's id is a stand-in.
    let placement = Placement {
        agent_id: AgentId::generate(),
        role,
        parent_agent_id: place.map(|place| place.parent_agent_id),
        team_id: place.map(|place| place.team_id),
    };
    check_placement(&snapshot.rows, placement)
        .map_err(|rule| TemplateCreateError::from_tree_rule(&rule))?;
    Ok(PlannedPlace {
        role,
        parent: resolved_parent(&snapshot.rows, placement),
        team: placement.team_id,
    })
}

/// Every place of the team passes the tree rules, on a copy of the tree that
/// gets the planned agents one after the other (D5).
fn check_team_places(
    snapshot: &TreeSnapshot,
    parent: AgentId,
    lead: &AgentSeed,
    members: &[AgentSeed],
) -> Result<(), TemplateCreateError> {
    let team_id = TeamId::generate();
    let mut rows = snapshot.rows.clone();
    let lead_id = AgentId::generate();
    let lead_place = Placement {
        agent_id: lead_id,
        role: Role::Lead,
        parent_agent_id: Some(parent),
        team_id: Some(team_id),
    };
    add_planned_row(&mut rows, &lead.name, lead_place)?;
    for member in members {
        let member_place = Placement {
            agent_id: AgentId::generate(),
            role: Role::Member,
            parent_agent_id: Some(lead_id),
            team_id: Some(team_id),
        };
        add_planned_row(&mut rows, &member.name, member_place)?;
    }
    Ok(())
}

fn add_planned_row(
    rows: &mut Vec<TreeRow>,
    name: &str,
    placement: Placement,
) -> Result<(), TemplateCreateError> {
    check_placement(rows, placement).map_err(|rule| TemplateCreateError::from_tree_rule(&rule))?;
    let parent_agent_id = resolved_parent(rows, placement);
    rows.push(TreeRow {
        agent_id: placement.agent_id,
        name: name.to_owned(),
        role: Some(placement.role),
        parent_agent_id,
        team_id: placement.team_id,
    });
    Ok(())
}

#[cfg(test)]
#[path = "template_create.tests.rs"]
mod tests;

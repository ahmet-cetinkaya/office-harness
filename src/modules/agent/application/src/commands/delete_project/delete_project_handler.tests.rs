// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use crate::common::interfaces::AgentStore;
use crate::test_support::{MemoryAgents, MemoryProjects, agent_in_project, project_named};

#[test]
fn an_unused_project_is_deleted() {
    let projects = Arc::new(MemoryProjects::new(Arc::new(MemoryAgents::default())));
    let project = project_named("Docs");
    projects.insert(&project).expect("insert");

    DeleteProjectHandler::new(projects.clone())
        .handle(project.id)
        .expect("delete");

    assert_eq!(projects.all().len(), 0);
}

#[test]
fn a_project_in_use_conflicts_and_names_its_agents() {
    let agents = Arc::new(MemoryAgents::default());
    let projects = Arc::new(MemoryProjects::new(agents.clone()));
    let project = project_named("Docs");
    projects.insert(&project).expect("insert");
    agents
        .insert(&agent_in_project("Ana", project.id))
        .expect("agent");

    let error = DeleteProjectHandler::new(projects.clone())
        .handle(project.id)
        .expect_err("in use");

    assert!(matches!(&error, ApplicationError::Conflict(text) if text.contains("Ana")));
    assert_eq!(projects.all().len(), 1);
}

#[test]
fn an_unknown_project_is_not_found() {
    let projects = Arc::new(MemoryProjects::new(Arc::new(MemoryAgents::default())));

    let error = DeleteProjectHandler::new(projects)
        .handle(ProjectId::generate())
        .expect_err("unknown");

    assert!(matches!(error, ApplicationError::NotFound(_)));
}

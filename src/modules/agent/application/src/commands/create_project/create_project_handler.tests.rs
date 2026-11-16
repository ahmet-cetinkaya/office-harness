// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use crate::test_support::{MemoryAgents, MemoryProjects, absolute_folder};

fn handler() -> (CreateProjectHandler, Arc<MemoryProjects>) {
    let projects = Arc::new(MemoryProjects::new(Arc::new(MemoryAgents::default())));
    (CreateProjectHandler::new(projects.clone()), projects)
}

fn request(name: &str) -> CreateProjectRequest {
    CreateProjectRequest {
        name: name.to_owned(),
        folder: absolute_folder("office"),
    }
}

#[test]
fn a_valid_project_is_stored() {
    let (handler, projects) = handler();

    let project = handler.handle(request("Docs")).expect("create");

    assert_eq!(projects.all(), [project]);
}

#[test]
fn a_blank_name_is_invalid_and_names_the_field() {
    let (handler, _) = handler();

    let error = handler.handle(request("  ")).expect_err("blank");

    assert!(matches!(&error, ApplicationError::Invalid(text) if text.contains("project name")));
}

#[test]
fn a_relative_folder_is_invalid_and_names_the_field() {
    let (handler, _) = handler();
    let mut relative = request("Docs");
    relative.folder = PathBuf::from("docs");

    let error = handler.handle(relative).expect_err("relative");

    assert!(matches!(&error, ApplicationError::Invalid(text) if text.contains("project folder")));
}

#[test]
fn a_name_that_differs_only_in_case_conflicts() {
    let (handler, _) = handler();
    handler.handle(request("Docs")).expect("first");

    let error = handler.handle(request("dOCS")).expect_err("duplicate");

    assert!(matches!(&error, ApplicationError::Conflict(text) if text.contains("project name")));
}

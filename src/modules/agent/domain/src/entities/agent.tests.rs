// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn agent_with(
    working_folder: Option<PathBuf>,
    project_id: Option<ProjectId>,
) -> Result<Agent, AgentError> {
    Agent::new(
        AgentId::generate(),
        DisplayName::parse("name", "Ana").expect("valid"),
        AgentCommand::parse("claude --verbose").expect("valid"),
        working_folder,
        project_id,
        MachineId::generate(),
        Timestamp::from_unix_millis(1),
    )
}

#[cfg(unix)]
const ABSOLUTE_FOLDER: &str = "/work/ana";
#[cfg(windows)]
const ABSOLUTE_FOLDER: &str = "C:\\work\\ana";

#[test]
fn a_new_agent_shows_stopped() {
    let agent = agent_with(Some(PathBuf::from(ABSOLUTE_FOLDER)), None).expect("valid");
    assert_eq!(agent.status, AgentStatus::Stopped);
}

#[test]
fn needs_a_working_folder_or_a_project() {
    assert_eq!(
        agent_with(None, None),
        Err(AgentError::NoWorkingFolderOrProject)
    );
    assert!(agent_with(None, Some(ProjectId::generate())).is_ok());
}

#[test]
fn rejects_a_relative_working_folder() {
    assert_eq!(
        agent_with(Some(PathBuf::from("work/ana")), None),
        Err(AgentError::RelativePath {
            field: "working folder"
        })
    );
}

#[test]
fn applies_events_through_the_state_machine() {
    let mut agent = agent_with(None, Some(ProjectId::generate())).expect("valid");

    agent.apply(StatusEvent::Start).expect("start");
    agent.apply(StatusEvent::FirstOutput).expect("output");
    assert_eq!(agent.status, AgentStatus::Working);

    assert!(agent.apply(StatusEvent::Start).is_err());
    assert_eq!(agent.status, AgentStatus::Working);
}

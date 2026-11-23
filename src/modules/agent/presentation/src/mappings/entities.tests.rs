// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use agent_domain::entities::FinishedReport;
use agent_domain::value_objects::{FinishedTempId, Objective};
use kernel::{Actor, AgentId, Timestamp};

use super::*;

fn mark(creator: Actor) -> TempMark {
    TempMark {
        objective: Objective::new("fix the failing build").expect("objective"),
        creator,
    }
}

#[test]
fn a_temp_shows_its_objective_and_the_creator_as_user_or_agent_id() {
    let agent_id = AgentId::generate();

    assert_eq!(
        temp_json(&mark(Actor::User)),
        json!({"objective": "fix the failing build", "creator": "user"})
    );
    assert_eq!(
        temp_json(&mark(Actor::Agent(agent_id)))["creator"],
        agent_id.to_string()
    );
}

#[test]
fn a_finished_entry_shows_every_field_and_nulls_for_the_absent_ones() {
    let id = FinishedTempId::generate();
    let agent_id = AgentId::generate();
    let mut entry = FinishedTemp {
        id,
        agent_id,
        name: "Ana".to_owned(),
        objective: "fix the failing build".to_owned(),
        creator: Actor::User,
        parent_name: None,
        started_at: Timestamp::from_unix_millis(1),
        ended_at: Timestamp::from_unix_millis(2),
        reason: "the person ended it".to_owned(),
        report: None,
        kept_folder: None,
        kept_branch: None,
        removal_failure: None,
    };

    assert_eq!(
        finished_temp_json(&entry),
        json!({
            "id": id.to_string(),
            "agent_id": agent_id.to_string(),
            "name": "Ana",
            "objective": "fix the failing build",
            "creator": "user",
            "parent_name": null,
            "started_at": 1,
            "ended_at": 2,
            "reason": "the person ended it",
            "report": null,
            "kept_folder": null,
            "kept_branch": null,
            "removal_failure": null,
        })
    );

    entry.parent_name = Some("Lead".to_owned());
    entry.report = Some(FinishedReport {
        kind: "done".to_owned(),
        text: "all green".to_owned(),
    });
    entry.kept_folder = Some(PathBuf::from("/data/worktrees/0a1b"));
    entry.kept_branch = Some("office-harness/ana".to_owned());
    entry.removal_failure = Some("git refused".to_owned());
    let body = finished_temp_json(&entry);
    assert_eq!(body["parent_name"], "Lead");
    assert_eq!(body["report"], json!({"kind": "done", "text": "all green"}));
    assert_eq!(body["kept_folder"], "/data/worktrees/0a1b");
    assert_eq!(body["kept_branch"], "office-harness/ana");
    assert_eq!(body["removal_failure"], "git refused");
}

#[test]
fn an_agent_without_a_mark_shows_a_null_temp() {
    let agent = agent_in_nowhere();
    assert_eq!(agent_json(&agent)["temp"], Value::Null);
}

fn agent_in_nowhere() -> Agent {
    use agent_domain::value_objects::{AgentCommand, DisplayName, MachineId};
    Agent::new(
        AgentId::generate(),
        DisplayName::parse("name", "Ana").expect("name"),
        AgentCommand::parse("claude").expect("command"),
        Some(std::env::temp_dir()),
        None,
        MachineId::generate(),
        Timestamp::from_unix_millis(0),
    )
    .expect("agent")
}

// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn frame(event: &Event) -> Value {
    serde_json::from_str(&event.encode()).expect("json")
}

#[test]
fn a_status_event_names_the_agent_and_the_status_and_may_carry_a_detail() {
    let agent_id = AgentId::generate();

    let plain = frame(&agent_status_event(
        agent_id,
        AgentStatus::WaitingForInput,
        None,
    ));
    let failed = frame(&agent_status_event(
        agent_id,
        AgentStatus::Failed,
        Some("no such command"),
    ));

    assert_eq!(plain["type"], "agent.status");
    assert_eq!(plain["agent_id"], agent_id.to_string());
    assert_eq!(plain["status"], "waiting_for_input");
    assert_eq!(plain.get("detail"), None);
    assert_eq!(failed["detail"], "no such command");
}

#[test]
fn a_deleted_event_names_the_agent() {
    let agent_id = AgentId::generate();

    let event = frame(&agent_deleted_event(agent_id));

    assert_eq!(event["type"], "agent.deleted");
    assert_eq!(event["agent_id"], agent_id.to_string());
}

#[test]
fn a_line_event_names_the_agent_and_carries_the_line() {
    let agent_id = AgentId::generate();
    let event = frame(&agent_line_event(agent_id, "100%"));
    assert_eq!(event["type"], "agent.line");
    assert_eq!(event["agent_id"], agent_id.to_string());
    assert_eq!(event["line"], "100%");
}

#[test]
fn an_additions_event_names_the_agent_and_carries_the_additions() {
    use agent_domain::entities::AdditionsRecord;
    use agent_domain::value_objects::{EnvEntry, OfficeAdditions};

    let agent_id = AgentId::generate();
    let view = AdditionsView {
        record: AdditionsRecord {
            additions: OfficeAdditions::new(
                vec!["--x".to_owned()],
                vec![EnvEntry::parse("KEY", "v").expect("entry")],
                Some("go".to_owned()),
            )
            .expect("additions"),
            updated_at: kernel::Timestamp::from_unix_millis(7),
        },
        applies_next_session: true,
    };

    let event = frame(&additions_changed_event(agent_id, &view));

    assert_eq!(event["type"], "agent.additions.changed");
    assert_eq!(event["agent_id"], agent_id.to_string());
    assert_eq!(event["additions"]["flags"][0], "--x");
    assert_eq!(event["additions"]["env"][0]["name"], "KEY");
    assert_eq!(event["additions"]["startup_prompt"], "go");
    assert_eq!(event["additions"]["applies_next_session"], true);
}

#[test]
fn a_project_changed_event_names_the_cause_and_the_project() {
    let project_id = ProjectId::generate();

    let event = frame(&project_changed_event("created", project_id));

    assert_eq!(event["type"], "project.changed");
    assert_eq!(event["cause"], "created");
    assert_eq!(event["project_id"], project_id.to_string());
}

#[test]
fn a_task_event_carries_the_task_or_null_and_no_line_or_status() {
    let agent_id = AgentId::generate();
    let task = agent_domain::value_objects::TaskText::from_input(
        "Fix the login test",
        kernel::Timestamp::from_unix_millis(9),
        agent_domain::value_objects::TaskSource::Directive,
    )
    .expect("valid");

    let set = frame(&agent_task_event(agent_id, task.as_ref()));
    let cleared = frame(&agent_task_event(agent_id, None));

    assert_eq!(set["type"], "agent.task");
    assert_eq!(set["agent_id"], agent_id.to_string());
    assert_eq!(set["task"]["text"], "Fix the login test");
    assert_eq!(set["task"]["set_at"], 9);
    assert_eq!(set["task"]["source"], "directive");
    assert_eq!(set.get("status"), None);
    assert_eq!(set.get("last_line"), None);
    assert_eq!(cleared["task"], Value::Null);
}

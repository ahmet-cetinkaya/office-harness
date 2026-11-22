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
                vec![EnvEntry::parse("KEY", Some("v")).expect("entry")],
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

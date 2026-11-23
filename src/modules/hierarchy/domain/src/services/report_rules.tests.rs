// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

use crate::value_objects::Role;

const CEO: u128 = 1;
const LEAD: u128 = 2;
const MEMBER: u128 = 3;
const UNPLACED: u128 = 4;

fn row(agent: u128, role: Role, parent: Option<u128>) -> TreeRow {
    TreeRow {
        agent_id: AgentId::from_u128(agent),
        name: agent.to_string(),
        role: Some(role),
        parent_agent_id: parent.map(AgentId::from_u128),
        team_id: None,
    }
}

fn company() -> Vec<TreeRow> {
    vec![
        row(CEO, Role::Ceo, None),
        row(LEAD, Role::Lead, Some(CEO)),
        row(MEMBER, Role::Member, Some(LEAD)),
    ]
}

#[test]
fn each_wire_kind_reads_back_and_an_unknown_kind_is_refused() {
    for kind in ReportKind::ALL {
        assert_eq!(ReportKind::from_wire(kind.as_str()), Ok(kind));
    }

    let refused = ReportKind::from_wire("finished-and-a-very-long-word");

    assert_eq!(
        refused,
        Err(HierarchyError::UnknownReportKind {
            text: "finished-and-a-v".to_owned()
        })
    );
}

#[test]
fn report_length_bounds_hold() {
    let at_limit = "x".repeat(REPORT_MAX_CHARS);
    let over = "é".repeat(REPORT_MAX_CHARS + 1);

    assert_eq!(validate_report_text(&at_limit), Ok(()));
    assert_eq!(
        validate_report_text(&over),
        Err(HierarchyError::TooLong {
            field: "text",
            max: REPORT_MAX_CHARS
        })
    );
}

#[test]
fn a_blank_report_is_refused() {
    assert_eq!(
        validate_report_text(" \n\t "),
        Err(HierarchyError::Empty { field: "text" })
    );
}

#[test]
fn control_characters_are_allowed_in_a_report() {
    assert_eq!(
        validate_report_text("line one\nline two \u{1b}[31m \u{202e}"),
        Ok(())
    );
}

#[test]
fn a_member_and_a_lead_report_to_their_parent() {
    let tree = company();

    assert_eq!(
        report_receiver(&tree, AgentId::from_u128(MEMBER), false),
        ReportReceiver::Parent(AgentId::from_u128(LEAD))
    );
    assert_eq!(
        report_receiver(&tree, AgentId::from_u128(LEAD), false),
        ReportReceiver::Parent(AgentId::from_u128(CEO))
    );
}

#[test]
fn an_agent_without_a_parent_reports_to_the_person() {
    let tree = company();

    assert_eq!(
        report_receiver(&tree, AgentId::from_u128(CEO), false),
        ReportReceiver::Person
    );
    assert_eq!(
        report_receiver(&tree, AgentId::from_u128(UNPLACED), false),
        ReportReceiver::Person
    );
}

#[test]
fn a_report_marked_for_the_person_skips_the_parent() {
    assert_eq!(
        report_receiver(&company(), AgentId::from_u128(MEMBER), true),
        ReportReceiver::Person
    );
}

#[test]
fn the_line_starts_with_the_heading_and_names_the_sender() {
    let sender = AgentId::from_u128(MEMBER);

    let line = report_line("M1", sender, ReportKind::Done, "tests pass");

    assert_eq!(
        line,
        format!("[report from M1 ({sender}): done] tests pass")
    );
    assert_eq!(
        report_record_text(ReportKind::Question, "which branch?"),
        "question: which branch?"
    );
}

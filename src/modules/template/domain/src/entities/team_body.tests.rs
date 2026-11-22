// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use crate::entities::{AdditionsBody, EnvVariable};
use crate::value_objects::NamePattern;

fn agent(pattern: &str, command: &str) -> AgentBody {
    AgentBody {
        name_pattern: NamePattern::new(pattern),
        command: command.to_owned(),
        working_folder: None,
        additions: AdditionsBody::default(),
        role: None,
    }
}

fn team(counts: &[u32]) -> TeamBody {
    TeamBody {
        lead: agent("{team}-lead", "claude"),
        members: counts
            .iter()
            .map(|count| MemberEntry {
                entry: agent("{team}-member-{n}", "claude"),
                count: *count,
            })
            .collect(),
    }
}

#[test]
fn fifty_agents_are_accepted_and_fifty_one_are_refused() {
    assert_eq!(team(&[30, 19]).validate(), Ok(()));
    assert_eq!(
        team(&[30, 20]).validate(),
        Err(TemplateError::TooManyAgents { total: 51, max: 50 })
    );
}

#[test]
fn a_team_needs_a_member_entry_and_each_count_is_at_least_one() {
    assert_eq!(team(&[]).validate(), Err(TemplateError::NoMembers));
    assert_eq!(
        team(&[2, 0]).validate(),
        Err(TemplateError::CountBelowOne { index: 1 })
    );
}

#[test]
fn an_entry_with_an_empty_command_or_pattern_is_named() {
    let mut body = team(&[1]);
    body.lead.command = " ".to_owned();
    assert_eq!(
        body.validate(),
        Err(TemplateError::CommandEmpty {
            place: "lead".to_owned()
        })
    );

    let mut body = team(&[1]);
    body.members[0].entry.name_pattern = NamePattern::new("");
    assert_eq!(
        body.validate(),
        Err(TemplateError::PatternEmpty {
            place: "members[0].entry".to_owned()
        })
    );
}

#[test]
fn a_pattern_longer_than_a_name_can_be_is_refused() {
    let long = "x".repeat(65);
    let body = agent(&long, "claude");

    assert!(matches!(
        body.validate("body"),
        Err(TemplateError::PatternTooLong { .. })
    ));
    assert_eq!(agent(&"x".repeat(64), "claude").validate("body"), Ok(()));
}

#[test]
fn dropping_environment_values_keeps_the_names() {
    let mut body = team(&[1]);
    body.lead.additions.env.push(EnvVariable {
        name: "KEY".to_owned(),
        value: Some("secret".to_owned()),
    });

    let dropped = body.without_env_values();

    assert_eq!(dropped.lead.additions.env[0].name, "KEY");
    assert_eq!(dropped.lead.additions.env[0].value, None);
}

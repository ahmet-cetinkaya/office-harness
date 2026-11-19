// SPDX-License-Identifier: AGPL-3.0-or-later

use hierarchy_domain::entities::TeamId;
use hierarchy_domain::{Role, Team, TeamName, TreeRow};
use kernel::AgentId;

use super::{RowPlace, assemble, row_places};

fn row(
    agent_id: AgentId,
    role: Option<Role>,
    parent: Option<AgentId>,
    team: Option<TeamId>,
) -> TreeRow {
    TreeRow {
        agent_id,
        name: agent_id.to_string(),
        role,
        parent_agent_id: parent,
        team_id: team,
    }
}

fn team(id: TeamId, name: &str, seat: AgentId) -> Team {
    Team::stored(id, TeamName::parse(name).expect("team name"), None, seat)
}

/// The tree finds a lead's team through the lead's own row, so a team row
/// whose seat column still names an earlier lead does not hide the team
/// (the stale-seat case after a lead moves).
#[test]
fn a_lead_shows_on_its_rows_team_even_when_the_seat_column_is_stale() {
    let lead = AgentId::from_u128(1);
    let member = AgentId::from_u128(2);
    let stale_seat = AgentId::from_u128(3);
    let team_id = TeamId::from_u128(10);
    let rows = [
        row(lead, Some(Role::Lead), None, Some(team_id)),
        row(member, Some(Role::Member), Some(lead), Some(team_id)),
    ];

    let tree = assemble(&rows, &[team(team_id, "Backend", stale_seat)]);

    assert_eq!(tree.teams.len(), 1);
    assert_eq!(tree.teams[0].team.id(), team_id);
    assert_eq!(tree.teams[0].lead.agent_id, lead);
    assert_eq!(tree.teams[0].members.len(), 1);
    assert_eq!(tree.teams[0].members[0].agent_id, member);
}

/// A row's place carries the role and the team's current name; a placed
/// agent without a team carries no name.
#[test]
fn row_places_name_each_rows_team() {
    let ceo = AgentId::from_u128(1);
    let lead = AgentId::from_u128(2);
    let team_id = TeamId::from_u128(10);
    let rows = [
        row(ceo, Some(Role::Ceo), None, None),
        row(lead, Some(Role::Lead), Some(ceo), Some(team_id)),
    ];

    let places = row_places(&rows, &[team(team_id, "Backend", lead)]);

    assert_eq!(
        places.get(&ceo),
        Some(&RowPlace {
            role: Some(Role::Ceo),
            team_name: None
        })
    );
    assert_eq!(
        places.get(&lead),
        Some(&RowPlace {
            role: Some(Role::Lead),
            team_name: Some("Backend".to_owned())
        })
    );
}

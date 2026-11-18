// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

use crate::entities::TeamId;
use crate::value_objects::Role;

#[derive(Clone, Copy)]
struct Ids {
    caller: AgentId,
    middle: AgentId,
    leaf: AgentId,
    outsider: AgentId,
}

fn fixed_ids() -> Ids {
    Ids {
        caller: AgentId::from_u128(1),
        middle: AgentId::from_u128(2),
        leaf: AgentId::from_u128(3),
        outsider: AgentId::from_u128(4),
    }
}

/// The name a test row carries: the id text, since these rules do not read
/// names.
fn name_of(agent_id: AgentId) -> String {
    agent_id.to_string()
}

fn tree() -> Vec<TreeRow> {
    let ids = fixed_ids();
    let team = TeamId::from_u128(10);

    vec![
        TreeRow {
            agent_id: ids.caller,
            name: name_of(ids.caller),
            role: Some(Role::Lead),
            parent_agent_id: None,
            team_id: Some(team),
        },
        TreeRow {
            agent_id: ids.middle,
            name: name_of(ids.middle),
            role: Some(Role::Member),
            parent_agent_id: Some(ids.caller),
            team_id: Some(team),
        },
        TreeRow {
            agent_id: ids.leaf,
            name: name_of(ids.leaf),
            role: Some(Role::Member),
            parent_agent_id: Some(ids.middle),
            team_id: Some(team),
        },
        TreeRow {
            agent_id: ids.outsider,
            name: name_of(ids.outsider),
            role: Some(Role::Member),
            parent_agent_id: None,
            team_id: Some(team),
        },
    ]
}

fn refusal(actor: AgentId) -> HierarchyError {
    HierarchyError::NotADescendant {
        actor: actor.to_string(),
    }
}

/// The `NotADescendant` failure, wrapped as the functions' `Result`.
fn refused<T>(actor: AgentId) -> Result<T, HierarchyError> {
    Err(refusal(actor))
}

#[test]
fn a_direct_descendant_passes() {
    let ids = fixed_ids();
    assert_eq!(check_descendant(&tree(), ids.caller, ids.middle), Ok(()));
}

#[test]
fn two_levels_deep_passes() {
    let ids = fixed_ids();
    assert_eq!(check_descendant(&tree(), ids.caller, ids.leaf), Ok(()));
}

#[test]
fn the_caller_itself_is_refused() {
    let ids = fixed_ids();
    assert_eq!(
        check_descendant(&tree(), ids.caller, ids.caller),
        refused(ids.caller)
    );
}

#[test]
fn an_unrelated_agent_is_refused() {
    let ids = fixed_ids();
    assert_eq!(
        check_descendant(&tree(), ids.caller, ids.outsider),
        refused(ids.caller)
    );
}

#[test]
fn an_upward_chain_is_refused() {
    let ids = fixed_ids();
    assert_eq!(
        check_descendant(&tree(), ids.middle, ids.caller),
        refused(ids.middle)
    );
}

#[test]
fn creation_defaults_to_the_caller() {
    let ids = fixed_ids();
    assert_eq!(creation_parent(&tree(), ids.caller, None), Ok(ids.caller));
}

#[test]
fn a_requested_descendant_lead_is_kept() {
    let ids = fixed_ids();
    assert_eq!(
        creation_parent(&tree(), ids.caller, Some(ids.middle)),
        Ok(ids.middle)
    );
    assert_eq!(
        creation_parent(&tree(), ids.caller, Some(ids.leaf)),
        Ok(ids.leaf)
    );
    // The caller itself stays a valid request.
    assert_eq!(
        creation_parent(&tree(), ids.caller, Some(ids.caller)),
        Ok(ids.caller)
    );
}

#[test]
fn an_outside_requested_parent_is_refused() {
    let ids = fixed_ids();
    assert_eq!(
        creation_parent(&tree(), ids.caller, Some(ids.outsider)),
        refused(ids.caller)
    );
    // The other direction refuses too: a caller below the request.
    assert_eq!(
        creation_parent(&tree(), ids.middle, Some(ids.caller)),
        refused(ids.middle)
    );
}

#[test]
fn plain_and_unicode_directive_text_passes() {
    assert_eq!(validate_directive_text("ship the report"), Ok(()));
    assert_eq!(validate_directive_text("merhaba, ça va? 🎉"), Ok(()));
}

#[test]
fn blank_directive_text_is_refused() {
    assert_eq!(
        validate_directive_text(""),
        Err(HierarchyError::Empty { field: "text" })
    );
    assert_eq!(
        validate_directive_text("   "),
        Err(HierarchyError::Empty { field: "text" })
    );
}

#[test]
fn directive_length_bounds_hold() {
    let at_limit = "x".repeat(DIRECTIVE_MAX_CHARS);
    assert_eq!(validate_directive_text(&at_limit), Ok(()));
    let over = "x".repeat(DIRECTIVE_MAX_CHARS + 1);
    assert_eq!(
        validate_directive_text(&over),
        Err(HierarchyError::TooLong {
            field: "text",
            max: DIRECTIVE_MAX_CHARS
        })
    );
}

#[test]
fn control_characters_are_refused_one_by_one() {
    let forbidden = ['\n', '\t', '\u{1b}', '\u{7f}', '\u{85}', '\u{9f}'];
    for character in forbidden {
        let text = format!("before{character}after");
        assert_eq!(
            validate_directive_text(&text),
            Err(HierarchyError::ControlCharacter { field: "text" }),
            "U+{:04X} must be refused",
            character as u32
        );
    }
}

#[test]
fn a_zero_width_joiner_in_an_emoji_passes() {
    let family = "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f466} ok";
    assert_eq!(validate_directive_text(family), Ok(()));
}

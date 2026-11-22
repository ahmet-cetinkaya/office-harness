// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::{AgentCommand, DisplayName, MachineId, TaskSource};
use hierarchy_domain::{HoldScope, HoldSetter};

const SET_AT: Timestamp = Timestamp::from_unix_millis(1_700_000_000_000);

/// The only keys an entry may carry (contract, FR-003).
const ENTRY_KEYS: [&str; 9] = [
    "agent_id",
    "held",
    "last_output_at",
    "name",
    "needs_you",
    "parent_agent_id",
    "role",
    "status",
    "task",
];

struct Office {
    ceo: AgentId,
    lead_a: AgentId,
    member_a: AgentId,
    lead_b: AgentId,
    member_b: AgentId,
    rows: Vec<TreeRow>,
    agents: Vec<Agent>,
}

fn row(agent_id: AgentId, role: Role, parent: Option<AgentId>) -> TreeRow {
    TreeRow {
        agent_id,
        name: agent_id.to_string(),
        role: Some(role),
        parent_agent_id: parent,
        team_id: None,
    }
}

fn agent(agent_id: AgentId, name: &str, status: AgentStatus) -> Agent {
    let mut agent = Agent::new(
        agent_id,
        DisplayName::parse("name", name).expect("name"),
        AgentCommand::parse("secret-cli --token abc").expect("command"),
        Some(std::env::temp_dir()),
        None,
        MachineId::generate(),
        SET_AT,
    )
    .expect("agent");
    agent.status = status;
    agent
}

/// A CEO over two leads, each with one member.
fn office() -> Office {
    let [ceo, lead_a, member_a, lead_b, member_b] = [1, 2, 3, 4, 5].map(AgentId::from_u128);
    let rows = vec![
        row(ceo, Role::Ceo, None),
        row(lead_a, Role::Lead, Some(ceo)),
        row(lead_b, Role::Lead, Some(ceo)),
        row(member_a, Role::Member, Some(lead_a)),
        row(member_b, Role::Member, Some(lead_b)),
    ];
    let agents = vec![
        agent(ceo, "Ceo", AgentStatus::Working),
        agent(lead_a, "Lead A", AgentStatus::Working),
        agent(member_a, "Member A", AgentStatus::WaitingForInput),
        agent(lead_b, "Lead B", AgentStatus::Stopped),
        agent(member_b, "Member B", AgentStatus::Working),
    ];
    Office {
        ceo,
        lead_a,
        member_a,
        lead_b,
        member_b,
        rows,
        agents,
    }
}

fn hold_on(scope: HoldScope) -> Hold {
    Hold {
        scope,
        set_by: HoldSetter::User,
        since: SET_AT,
        reason: None,
    }
}

fn entries_of(office: &Office, caller: AgentId, waiting: &[AgentId], holds: &[Hold]) -> Vec<Value> {
    let waiting: HashSet<AgentId> = waiting.iter().copied().collect();
    let state = SubtreeState {
        rows: &office.rows,
        agents: &office.agents,
        waiting: &waiting,
        holds,
    };
    subtree_entries(&state, caller, &|_| None)
}

fn ids_in(entries: &[Value]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| entry["agent_id"].as_str().expect("id").to_owned())
        .collect()
}

#[test]
fn the_ceo_reads_every_descendant_each_after_its_parent() {
    let office = office();
    let entries = entries_of(&office, office.ceo, &[], &[]);
    assert_eq!(
        ids_in(&entries),
        [
            office.lead_a,
            office.member_a,
            office.lead_b,
            office.member_b
        ]
        .map(|id| id.to_string())
    );
}

#[test]
fn a_lead_reads_its_own_branch_only() {
    let office = office();
    let entries = entries_of(&office, office.lead_a, &[], &[]);
    assert_eq!(ids_in(&entries), [office.member_a.to_string()]);
}

#[test]
fn a_member_reads_an_empty_list() {
    let office = office();
    assert_eq!(
        entries_of(&office, office.member_a, &[], &[]),
        Vec::<Value>::new()
    );
}

#[test]
fn an_entry_carries_exactly_the_contract_fields() {
    let mut office = office();
    office.agents[2].task =
        TaskText::from_input("Fix the login test", SET_AT, TaskSource::Directive).expect("text");
    let waiting: HashSet<AgentId> = HashSet::from([office.member_a]);
    let state = SubtreeState {
        rows: &office.rows,
        agents: &office.agents,
        waiting: &waiting,
        holds: &[],
    };
    let quiet_since = Timestamp::from_unix_millis(1_700_000_000_500);
    let entries = subtree_entries(&state, office.lead_a, &|_| Some(quiet_since));
    let entry = entries[0].as_object().expect("object");
    let mut keys: Vec<&str> = entry.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ENTRY_KEYS);
    assert_eq!(entry["name"], "Member A");
    assert_eq!(entry["parent_agent_id"], office.lead_a.to_string());
    assert_eq!(entry["role"], "member");
    assert_eq!(entry["status"], "waiting_for_input");
    assert_eq!(entry["last_output_at"], 1_700_000_000_500_i64);
    assert_eq!(entry["needs_you"], true);
    assert_eq!(entry["held"], false);
    assert_eq!(
        entry["task"],
        json!({"line": "Fix the login test", "set_at": SET_AT.as_unix_millis()})
    );
    let text = Value::Object(entry.clone()).to_string();
    assert!(!text.contains("secret-cli"), "{text}");
}

#[test]
fn a_hold_on_an_ancestor_or_the_office_covers_a_descendant() {
    let office = office();
    let lead_held = entries_of(
        &office,
        office.ceo,
        &[],
        &[hold_on(HoldScope::Agent(office.lead_a))],
    );
    let held: Vec<bool> = lead_held
        .iter()
        .map(|entry| entry["held"].as_bool().expect("flag"))
        .collect();
    assert_eq!(held, [true, true, false, false]);

    let office_held = entries_of(&office, office.lead_b, &[], &[hold_on(HoldScope::Office)]);
    assert_eq!(office_held[0]["held"], true);

    let caller_held = entries_of(
        &office,
        office.lead_b,
        &[],
        &[hold_on(HoldScope::Agent(office.lead_b))],
    );
    assert_eq!(caller_held[0]["held"], true);
}

#[test]
fn a_descendant_without_a_task_or_a_running_child_shows_null() {
    let office = office();
    let entry = &entries_of(&office, office.lead_b, &[], &[])[0];
    assert_eq!(entry["task"], Value::Null);
    assert_eq!(entry["last_output_at"], Value::Null);
    assert_eq!(entry["needs_you"], false);
}

#[test]
fn a_descendant_the_agents_no_longer_hold_is_left_out() {
    let mut office = office();
    office.agents.retain(|agent| agent.id != office.member_a);
    assert_eq!(
        entries_of(&office, office.lead_a, &[], &[]),
        Vec::<Value>::new()
    );
}

#[test]
fn the_task_line_keeps_eighty_characters_of_the_first_line() {
    let exactly = "a".repeat(80);
    assert_eq!(task_line(&exactly), exactly);
    let longer = "b".repeat(81);
    assert_eq!(task_line(&longer), format!("{}…", "b".repeat(80)));
    assert_eq!(task_line("first\nsecond"), "first…");
    assert_eq!(task_line("first\r\nsecond"), "first…");
    assert_eq!(task_line("only line\n"), "only line");
}

#[test]
fn control_and_bidirectional_characters_become_visible_symbols() {
    assert_eq!(visible_text("\u{1b}[31m"), "␛[31m");
    assert_eq!(visible_text("a\tb"), "a␉b");
    assert_eq!(visible_text("\u{7f}"), "␡");
    assert_eq!(visible_text("\u{9b}2J"), "\\u009B2J");
    assert_eq!(visible_text("\u{202e}evil"), "\\u202Eevil");
    assert_eq!(visible_text("\u{2066}x\u{2069}"), "\\u2066x\\u2069");
    assert_eq!(visible_text("plain text ✓"), "plain text ✓");
    assert_eq!(
        task_line("\u{1b}[31m\u{9b}\u{202e}red\nsecond"),
        "␛[31m\\u009B\\u202Ered…"
    );
}

#[test]
fn a_quiet_time_turns_into_the_time_of_the_last_output() {
    let now = Timestamp::from_unix_millis(10_000);
    assert_eq!(
        time_before(now, Duration::from_millis(2_500)),
        Timestamp::from_unix_millis(7_500)
    );
}

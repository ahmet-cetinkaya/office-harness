// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(unix)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! A running agent sends a directive to its member over the agent-call
//! helper (0002, T5 acceptance d, e, f). The member's fake CLI records the
//! bytes the terminal typed, so the test reads the directive back as one
//! input followed by the Enter the server appended (SC-006).

mod support;

use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::Arc;

use agent_infrastructure::persistence::SharedConnection;
use hierarchy_application::commands::place_agent::{PlaceAgentHandler, PlaceAgentRequest};
use hierarchy_application::common::interfaces::HierarchyStore;
use hierarchy_domain::Role;
use hierarchy_infrastructure::persistence::repositories::hierarchy_repository::HierarchyRepository;
use hierarchy_infrastructure::services::agent_places::AgentPlaces;
use kernel::AgentId;
use serde_json::{Value, json};
use support::office_client::{Client, Office, wait_for};

/// The real server binary that the fake CLIs run as the helper.
const HELPER: &str = env!("CARGO_BIN_EXE_office-harnessd");

/// The bytes one directive delivers: the text and the Enter (SC-006).
const DIRECTIVE: &str = "hello member";
const DIRECTIVE_BYTES: usize = DIRECTIVE.len() + 1;

/// A fake CLI that hands its environment to the records folder and stays
/// alive: the test then holds the credential of its live session.
fn env_cli(records: &Path) -> String {
    let folder = records.display();
    format!("env > {folder}/env.part\nmv {folder}/env.part {folder}/env\nsleep 30\n")
}

/// A fake CLI that hands its environment to the records folder, calls the
/// helper once with the given request, keeps the answer and exit code, and
/// stays alive for the test.
fn calling_cli(records: &Path, kind: &str, request: &Value) -> String {
    let folder = records.display();
    format!(
        "env > {folder}/env.part\nmv {folder}/env.part {folder}/env\n{HELPER} agent-call {kind} '{request}' > {folder}/call.out 2> {folder}/call.err\nprintf '%s' $? > {folder}/call.code\nsleep 30\n"
    )
}

/// Places an agent through the same store the server built: the wire has no
/// placement request yet, so the test writes the place directly, as
/// `agent_subtree.rs` does.
fn place(office: &Office, agent_id: &str, role: Role, parent: Option<&str>) {
    let connection =
        rusqlite::Connection::open(office.serving.office.config.database_path()).expect("database");
    let store: Arc<dyn HierarchyStore> = Arc::new(HierarchyRepository::new(
        SharedConnection::new(connection).handle(),
        AgentPlaces::new(),
    ));
    PlaceAgentHandler::new(store)
        .handle(PlaceAgentRequest {
            agent_id: agent_id.parse().expect("agent id"),
            role,
            parent_agent_id: parent.map(|id| id.parse().expect("agent id")),
            team_id: None,
        })
        .expect("placement");
}

/// The action entries the wire holds for `agent_id`.
async fn actions_of(client: &mut Client, agent_id: &str) -> Vec<Value> {
    let actions = client
        .ask("agent.actions", json!({"agent_id": agent_id}))
        .await;
    assert_eq!(actions["ok"], true, "{actions}");
    actions["result"].as_array().expect("entries").clone()
}

/// The live credential of the first started CLI, from the environment the
/// server gave its session.
async fn live_credential(records: &Path) -> (String, String) {
    let env = wait_for("the session's environment", || {
        fs::read_to_string(records.join("env")).ok()
    })
    .await;
    let value = |name: &str| {
        env.lines()
            .find_map(|line| line.strip_prefix(&format!("{name}=")))
            .expect(name)
            .to_owned()
    };
    (
        value("OFFICE_HARNESS_AGENT_TOKEN"),
        value("OFFICE_HARNESS_SERVER_FILE"),
    )
}

/// Runs `directive.send` through the helper.
async fn send_directive(credential: &str, server_file: &str, request: &Value) -> Output {
    call_helper("directive.send", credential, server_file, request).await
}

/// Runs the helper with a credential, on a blocking thread: waiting for it
/// on the runtime thread would stop the server it talks to.
async fn call_helper(kind: &str, credential: &str, server_file: &str, request: &Value) -> Output {
    let kind = kind.to_owned();
    let credential = credential.to_owned();
    let server_file = server_file.to_owned();
    let request = request.to_string();
    tokio::task::spawn_blocking(move || {
        Command::new(HELPER)
            .args(["agent-call", &kind, &request])
            .env("OFFICE_HARNESS_AGENT_TOKEN", credential)
            .env("OFFICE_HARNESS_SERVER_FILE", server_file)
            .output()
            .expect("helper")
    })
    .await
    .expect("helper task")
}

/// The entry of the next `action.recorded` event that the watcher receives
/// with this action on this agent. The watcher sent no request, so the event
/// can only arrive through the server's publish to every client.
async fn next_recorded(watcher: &mut Client, action: &str, agent_id: &str) -> Value {
    loop {
        let event = watcher.next_event("action.recorded").await;
        let entry = &event["entry"];
        if entry["action"] == action && entry["agent_id"] == agent_id {
            return entry.clone();
        }
    }
}

/// The JSON answer of a failed helper call.
fn answer_of(outcome: &Output) -> Value {
    serde_json::from_str(String::from_utf8_lossy(&outcome.stdout).trim()).expect("json")
}

/// Builds the office, a CEO over a placed or unplaced subordinate, both
/// started, and returns the caller's credential. The CEO's CLI is the
/// environment dumper, so the test holds its live session's credential.
async fn office_with(office: &Office, member: Option<Role>) -> (String, String, String, String) {
    office_with_member_cli(office, member, "echo ready; sleep 30").await
}

/// As `office_with`, with the subordinate's CLI given as a script.
async fn office_with_member_cli(
    office: &Office,
    member: Option<Role>,
    member_cli: &str,
) -> (String, String, String, String) {
    office.clis.add("member-cli", member_cli);
    office.clis.add("lead-cli", &env_cli(office.records_path()));
    let mut client = office.client().await;
    let lead = client.create_agent(office, "Lead", "lead-cli", None).await;
    let member_id = client
        .create_agent(office, "Member", "member-cli", None)
        .await;
    place(office, &lead, Role::Ceo, None);
    if let Some(role) = member {
        place(office, &member_id, role, Some(&lead));
    }
    let started = client
        .ask("agent.start", json!({"agent_id": member_id}))
        .await;
    assert_eq!(started["ok"], true, "{started}");
    client.wait_for_status(&member_id, "working").await;
    let lead_started = client.ask("agent.start", json!({"agent_id": lead})).await;
    assert_eq!(lead_started["ok"], true, "{lead_started}");
    let (token, server_file) = live_credential(office.records_path()).await;
    (lead, member_id, token, server_file)
}

#[tokio::test]
async fn a_lead_sends_a_directive_and_the_member_takes_it_as_one_input_with_enter() {
    let office = Office::open().await;
    let typed = office.records_path().join("typed");
    // The member reads exactly the bytes of one directive: the text and the
    // Enter the server appends. Its terminal echoes, as a CLI's input line
    // shows typed text, so the server sees the text and presses Enter
    // (0012/FR-003).
    office.clis.add(
        "member-cli",
        &format!(
            "echo ready\nstty raw echo\ndd bs=1 count={DIRECTIVE_BYTES} of={} 2>/dev/null\nsleep 30",
            typed.display()
        ),
    );
    let mut client = office.client().await;
    let member_id = client
        .create_agent(&office, "Member", "member-cli", None)
        .await;
    // The lead calls the helper itself, with its own session credential.
    let request = json!({"target_agent_id": member_id, "text": DIRECTIVE});
    office.clis.add(
        "lead-cli",
        &calling_cli(office.records_path(), "directive.send", &request),
    );
    let lead = client.create_agent(&office, "Lead", "lead-cli", None).await;
    place(&office, &lead, Role::Ceo, None);
    place(&office, &member_id, Role::Lead, Some(&lead));
    let member_started = client
        .ask("agent.start", json!({"agent_id": member_id}))
        .await;
    assert_eq!(member_started["ok"], true, "{member_started}");
    // The directive is typed at once only into a member that waits for
    // input (0012/FR-002).
    client
        .wait_for_status(&member_id, "waiting_for_input")
        .await;
    let lead_started = client.ask("agent.start", json!({"agent_id": lead})).await;
    assert_eq!(lead_started["ok"], true, "{lead_started}");

    // A redirect creates its file empty and fills it afterwards, so an
    // existing file is not a whole one: the wait needs content, not
    // existence.
    let code = wait_for("the helper's exit code", || {
        fs::read_to_string(office.records_path().join("call.code"))
            .ok()
            .filter(|text| !text.is_empty())
    })
    .await;
    let out = wait_for("the helper's answer", || {
        fs::read_to_string(office.records_path().join("call.out"))
            .ok()
            .filter(|text| !text.is_empty())
    })
    .await;
    assert_eq!(code, "0", "{out}");
    let answer: Value = serde_json::from_str(out.trim()).expect("json");
    assert_eq!(answer["ok"], true, "{answer}");
    assert_eq!(answer["result"]["delivery"], "submitted", "{answer}");

    // SC-006: the text arrives as one input followed by Enter, nothing
    // between.
    let received = wait_for("the typed directive", || {
        fs::read(&typed)
            .ok()
            .filter(|bytes| bytes.len() == DIRECTIVE_BYTES)
    })
    .await;
    assert_eq!(received.len(), DIRECTIVE_BYTES, "{received:?}");
    assert_eq!(String::from_utf8_lossy(&received), format!("{DIRECTIVE}\r"));

    // The delivered attempt is in the record as the lead's directive on the
    // member (FR-011); the record is kept per target agent.
    let entries = actions_of(&mut client, &member_id).await;
    let entry = entries
        .iter()
        .find(|entry| entry["action"] == "directive")
        .expect("the directive is recorded");
    assert_eq!(entry["result"], "ok", "{entry}");
    assert_eq!(entry["actor"], json!(format!("agent:{lead}")), "{entry}");
    assert_eq!(entry["agent_id"], member_id, "{entry}");
    client.ask("agent.stop", json!({"agent_id": lead})).await;
    client
        .ask("agent.stop", json!({"agent_id": member_id}))
        .await;
    office.serving.stop().await;
}

#[tokio::test]
async fn a_directive_to_a_running_non_descendant_is_refused_and_recorded() {
    let office = Office::open().await;
    // The stranger stays unplaced, so it is outside the lead's subtree.
    let (lead, stranger, token, server_file) = office_with(&office, None).await;

    let request = json!({"target_agent_id": stranger, "text": "you there?"});
    let outcome = send_directive(&token, &server_file, &request).await;
    assert!(!outcome.status.success(), "{outcome:?}");
    let answer = answer_of(&outcome);
    assert_eq!(answer["error"]["code"], "refused", "{answer}");
    assert!(
        answer["error"]["message"]
            .as_str()
            .expect("message")
            .contains("not a descendant"),
        "{answer}"
    );

    // The refusal is recorded for the sender (FR-009), kept on the target's
    // entry like every action.
    let mut client = office.client().await;
    let entries = actions_of(&mut client, &stranger).await;
    let entry = entries
        .iter()
        .find(|entry| entry["action"] == "directive")
        .expect("the refused attempt is recorded");
    assert_ne!(entry["result"], "ok", "{entry}");
    assert_eq!(entry["actor"], json!(format!("agent:{lead}")), "{entry}");
    assert_eq!(entry["agent_id"], stranger, "{entry}");
    office.serving.stop().await;
}

#[tokio::test]
async fn a_refused_directive_is_recorded_for_the_sender_and_for_the_target() {
    let office = Office::open().await;
    // The stranger stays unplaced, so the tree refuses the directive.
    let (lead, stranger, token, server_file) = office_with(&office, None).await;

    let request = json!({"target_agent_id": stranger, "text": "you there?"});
    let outcome = send_directive(&token, &server_file, &request).await;
    assert!(!outcome.status.success(), "{outcome:?}");

    // Both agents' records, read back, list the refusal once (ACF-168).
    let mut client = office.client().await;
    for agent in [&lead, &stranger] {
        let entries = actions_of(&mut client, agent).await;
        let refused = entries
            .iter()
            .filter(|entry| entry["action"] == "directive" && entry["result"] != "ok")
            .count();
        assert_eq!(refused, 1, "{agent}: {entries:?}");
    }
    office.serving.stop().await;
}

#[tokio::test]
async fn an_escape_in_the_text_is_invalid_and_recorded() {
    let office = Office::open().await;
    let (lead, member, token, server_file) = office_with(&office, Some(Role::Lead)).await;

    let request = json!({"target_agent_id": member, "text": "\u{1b}[D"});
    let outcome = send_directive(&token, &server_file, &request).await;
    assert!(!outcome.status.success(), "{outcome:?}");
    let answer = answer_of(&outcome);
    assert_eq!(answer["error"]["code"], "invalid", "{answer}");

    // The refused attempt is recorded with the text as it was sent (FR-011),
    // kept on the target's entry.
    let mut client = office.client().await;
    let entries = actions_of(&mut client, &member).await;
    let entry = entries
        .iter()
        .find(|entry| entry["action"] == "directive")
        .expect("the refused attempt is recorded");
    assert_ne!(entry["result"], "ok", "{entry}");
    assert_eq!(entry["actor"], json!(format!("agent:{lead}")), "{entry}");
    assert!(
        entry["result"]
            .as_str()
            .expect("result")
            .contains("control"),
        "{entry}"
    );
    office.serving.stop().await;
}

#[tokio::test]
async fn a_request_carrying_an_actor_field_is_refused_and_recorded() {
    let office = Office::open().await;
    let (lead, member, token, server_file) = office_with(&office, Some(Role::Lead)).await;

    let request = json!({
        "target_agent_id": member,
        "text": "hello",
        "actor": AgentId::generate().as_uuid_text(),
    });
    let outcome = send_directive(&token, &server_file, &request).await;
    assert!(!outcome.status.success(), "{outcome:?}");
    let answer = answer_of(&outcome);
    assert_eq!(answer["error"]["code"], "refused", "{answer}");
    assert!(
        answer["error"]["message"]
            .as_str()
            .expect("message")
            .contains("actor"),
        "{answer}"
    );

    // The gateway refusal is recorded too (FR-008).
    let mut client = office.client().await;
    let entries = actions_of(&mut client, &lead).await;
    let entry = entries
        .iter()
        .find(|entry| entry["action"] == "refused")
        .expect("the gateway refusal is recorded");
    assert_eq!(entry["actor"], json!(format!("agent:{lead}")), "{entry}");
    office.serving.stop().await;
}

#[tokio::test]
async fn a_refused_directive_of_an_agent_reaches_a_client_that_did_not_send_it() {
    let office = Office::open().await;
    let (lead, stranger, token, server_file) = office_with(&office, None).await;
    let mut watcher = office.client().await;

    let request = json!({"target_agent_id": stranger, "text": "you there?"});
    let outcome = send_directive(&token, &server_file, &request).await;
    assert_eq!(answer_of(&outcome)["error"]["code"], "refused");

    // The target's entry arrives; a second entry for the sender may come with
    // it, so the test looks for the target's.
    let entry = next_recorded(&mut watcher, "directive", &stranger).await;
    assert_ne!(entry["result"], "ok", "{entry}");
    assert_eq!(entry["actor"], json!(format!("agent:{lead}")), "{entry}");
    office.serving.stop().await;
}

#[tokio::test]
async fn an_accepted_directive_of_an_agent_reaches_a_client_that_did_not_send_it() {
    let office = Office::open().await;
    let (lead, member, token, server_file) = office_with(&office, Some(Role::Lead)).await;
    let mut watcher = office.client().await;

    let request = json!({"target_agent_id": member, "text": DIRECTIVE});
    let outcome = send_directive(&token, &server_file, &request).await;
    assert!(outcome.status.success(), "{outcome:?}");

    let entry = next_recorded(&mut watcher, "directive", &member).await;
    assert_eq!(entry["result"], "ok", "{entry}");
    assert_eq!(entry["actor"], json!(format!("agent:{lead}")), "{entry}");
    office.serving.stop().await;
}

#[tokio::test]
async fn a_gateway_refusal_of_an_agent_reaches_a_client_that_did_not_send_it() {
    let office = Office::open().await;
    let (lead, _member, token, server_file) = office_with(&office, Some(Role::Lead)).await;
    let mut watcher = office.client().await;

    // `agent.delete` is outside the agent allowlist, so the gateway itself
    // refuses it and writes the entry.
    let outcome = call_helper(
        "agent.delete",
        &token,
        &server_file,
        &json!({"agent_id": lead}),
    )
    .await;
    assert_eq!(answer_of(&outcome)["error"]["code"], "refused");

    let entry = next_recorded(&mut watcher, "refused", &lead).await;
    assert_eq!(entry["actor"], json!(format!("agent:{lead}")), "{entry}");
    office.serving.stop().await;
}

/// How many times a test reads the record again while it waits for an
/// entry the delivery thread writes.
const RECORD_READS: usize = 600;

/// The agent's directive entry whose result contains `needle`, read back
/// until the delivery thread has written it.
async fn failure_containing(client: &mut Client, agent_id: &str, needle: &str) -> Value {
    for _ in 0..RECORD_READS {
        let entries = actions_of(client, agent_id).await;
        let found = entries.into_iter().find(|entry| {
            entry["action"] == "directive"
                && entry["result"]
                    .as_str()
                    .is_some_and(|result| result.contains(needle))
        });
        if let Some(entry) = found {
            return entry;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("no directive entry of {agent_id} contains {needle:?}");
}

/// A permission question with a yes-or-no choice, as `printf` text.
const YES_NO_QUESTION: &str = "Do you want to proceed? (y/n)\\n";

/// A start-up question whose choices have no numbers, as `printf` text:
/// a marked entry, one more entry under it, and a key hint.
const UNNUMBERED_CHOICES: &str =
    " Pick a mode for this folder.\\n\\n ❯ Leave\\n   Stay and work here\\n\\n Enter to confirm\\n";

/// A member that shows the question for a while, notes whatever reached it
/// meanwhile, then clears the question, shows an input line with echo, and
/// reads one directive.
fn questioning_cli(records: &Path, question: &str) -> String {
    let folder = records.display();
    format!(
        "printf '{question}'\nsleep 4\nstty raw -echo min 0 time 1\ndd bs=64 count=1 of={folder}/early 2>/dev/null\nprintf '\\033[2J\\033[H> '\nstty raw echo min 1 time 0\ndd bs=1 count={DIRECTIVE_BYTES} of={folder}/typed 2>/dev/null\nsleep 30\n"
    )
}

#[tokio::test]
async fn a_directive_waits_while_a_question_shows_and_is_typed_after_it() {
    directive_waits_for_the_question_and_is_typed_after_it(YES_NO_QUESTION).await;
}

#[tokio::test]
async fn a_directive_waits_while_a_choice_list_without_numbers_shows_and_is_typed_after_it() {
    directive_waits_for_the_question_and_is_typed_after_it(UNNUMBERED_CHOICES).await;
}

async fn directive_waits_for_the_question_and_is_typed_after_it(question: &str) {
    let office = Office::open().await;
    let cli = questioning_cli(office.records_path(), question);
    let (_lead, member, token, server_file) =
        office_with_member_cli(&office, Some(Role::Lead), &cli).await;
    let mut client = office.client().await;
    client.wait_for_status(&member, "waiting_for_input").await;

    let request = json!({"target_agent_id": member, "text": DIRECTIVE});
    let outcome = send_directive(&token, &server_file, &request).await;
    assert!(outcome.status.success(), "{outcome:?}");
    let answer = answer_of(&outcome);
    assert_eq!(answer["result"]["delivery"], "waiting", "{answer}");
    assert_eq!(answer["result"]["position"], 1, "{answer}");

    // SC-001: nothing reached the member while the question showed; the
    // directive came after it, text and Enter.
    let typed = wait_for("the typed directive", || {
        fs::read(office.records_path().join("typed"))
            .ok()
            .filter(|bytes| bytes.len() == DIRECTIVE_BYTES)
    })
    .await;
    let early = fs::read(office.records_path().join("early")).expect("early read");
    assert!(early.is_empty(), "{early:?}");
    assert_eq!(String::from_utf8_lossy(&typed), format!("{DIRECTIVE}\r"));
    office.serving.stop().await;
}

#[tokio::test]
async fn a_text_that_does_not_show_gets_no_enter_and_holds_the_next_directive() {
    let office = Office::open().await;
    let typed = office.records_path().join("typed");
    // An unknown question that does not echo: the text cannot show.
    let cli = format!(
        "echo 'Pick a fruit'\nstty raw -echo\ndd bs=1 count={DIRECTIVE_BYTES} of={} 2>/dev/null\nsleep 30",
        typed.display()
    );
    let (lead, member, token, server_file) =
        office_with_member_cli(&office, Some(Role::Lead), &cli).await;
    let mut client = office.client().await;
    client.wait_for_status(&member, "waiting_for_input").await;

    let request = json!({"target_agent_id": member, "text": DIRECTIVE});
    let first = answer_of(&send_directive(&token, &server_file, &request).await);
    assert_eq!(first["result"]["delivery"], "unsubmitted", "{first}");
    // SC-002: the text, and no Enter.
    let received = wait_for("the typed text", || {
        fs::read(&typed)
            .ok()
            .filter(|bytes| bytes.len() == DIRECTIVE.len())
    })
    .await;
    assert_eq!(String::from_utf8_lossy(&received), DIRECTIVE);

    // The unsubmitted text holds the queue (0012 A11, SC-007).
    let second = answer_of(&send_directive(&token, &server_file, &request).await);
    assert_eq!(second["result"]["delivery"], "waiting", "{second}");

    // SC-005: both records carry the reason.
    for agent in [&lead, &member] {
        let entry = failure_containing(&mut client, agent, "not submitted").await;
        assert_eq!(entry["actor"], json!(format!("agent:{lead}")), "{entry}");
    }
    assert_eq!(fs::read(&typed).expect("typed").len(), DIRECTIVE.len());
    office.serving.stop().await;
}

#[tokio::test]
async fn a_startup_prompt_that_waits_for_the_person_holds_directives() {
    let office = Office::open().await;
    let typed = office.records_path().join("typed");
    let prompt = "review me";
    let cli = format!(
        "echo ready\nstty raw\ndd bs=1 count=64 of={} 2>/dev/null\nsleep 30",
        typed.display()
    );
    office.clis.add("member-cli", &cli);
    office.clis.add("lead-cli", &env_cli(office.records_path()));
    let mut client = office.client().await;
    let lead = client.create_agent(&office, "Lead", "lead-cli", None).await;
    let member = client
        .create_agent(&office, "Member", "member-cli", None)
        .await;
    place(&office, &lead, Role::Ceo, None);
    place(&office, &member, Role::Lead, Some(&lead));
    let set = client
        .ask(
            "agent.additions.set",
            json!({"agent_id": member, "additions": {"startup_prompt": prompt}}),
        )
        .await;
    assert_eq!(set["ok"], true, "{set}");
    client.start(&member).await;
    wait_for("the pasted startup prompt", || {
        fs::read_to_string(&typed)
            .ok()
            .filter(|text| text.contains(prompt))
    })
    .await;
    client.wait_for_status(&member, "waiting_for_input").await;
    client.start(&lead).await;
    let (token, server_file) = live_credential(office.records_path()).await;

    let request = json!({"target_agent_id": member, "text": DIRECTIVE});
    let answer = answer_of(&send_directive(&token, &server_file, &request).await);

    // SC-007: the directive waits, so no Enter submits the prompt.
    assert_eq!(answer["result"]["delivery"], "waiting", "{answer}");
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let received = fs::read_to_string(&typed).expect("typed");
    assert!(!received.contains(DIRECTIVE), "{received:?}");
    assert!(!received.contains('\r'), "{received:?}");
    office.serving.stop().await;
}

/// How long a test watches a terminal to see that nothing more arrives.
const NOTHING_MORE_WAIT: std::time::Duration = std::time::Duration::from_millis(1500);

#[tokio::test]
async fn a_half_typed_line_of_a_person_holds_a_directive_and_nothing_is_typed() {
    let office = Office::open().await;
    let typed = office.records_path().join("typed");
    let cli = format!("echo ready\nstty raw echo\ncat >> {}\n", typed.display());
    let (_lead, member, token, server_file) =
        office_with_member_cli(&office, Some(Role::Lead), &cli).await;
    let mut client = office.client().await;
    client.wait_for_status(&member, "waiting_for_input").await;
    // "hi" as a person types it, without Enter (0012 A5).
    let half_typed = client
        .ask(
            "terminal.input",
            json!({"agent_id": member, "data": "aGk="}),
        )
        .await;
    assert_eq!(half_typed["ok"], true, "{half_typed}");

    let request = json!({"target_agent_id": member, "text": DIRECTIVE});
    let answer = answer_of(&send_directive(&token, &server_file, &request).await);

    assert_eq!(answer["result"]["delivery"], "waiting", "{answer}");
    tokio::time::sleep(NOTHING_MORE_WAIT).await;
    let received = fs::read_to_string(&typed).expect("typed");
    assert_eq!(received, "hi");
    office.serving.stop().await;
}

#[tokio::test]
async fn a_waiting_directive_is_withdrawn_and_recorded_when_its_target_stops() {
    let office = Office::open().await;
    let cli = "echo 'Do you want to proceed? (y/n)'; sleep 30";
    let (lead, member, token, server_file) =
        office_with_member_cli(&office, Some(Role::Lead), cli).await;
    let mut client = office.client().await;
    client.wait_for_status(&member, "waiting_for_input").await;
    let request = json!({"target_agent_id": member, "text": DIRECTIVE});
    let answer = answer_of(&send_directive(&token, &server_file, &request).await);
    assert_eq!(answer["result"]["delivery"], "waiting", "{answer}");

    let stopped = client.ask("agent.stop", json!({"agent_id": member})).await;
    assert_eq!(stopped["ok"], true, "{stopped}");

    // FR-005, SC-005: in both records, with the text and the reason.
    for agent in [&lead, &member] {
        let entry = failure_containing(&mut client, agent, "withdrawn before typing").await;
        let result = entry["result"].as_str().expect("result");
        assert!(result.contains("no longer running"), "{entry}");
        assert!(result.contains(DIRECTIVE), "{entry}");
    }
    office.serving.stop().await;
}

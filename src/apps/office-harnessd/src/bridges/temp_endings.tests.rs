// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::common::errors::PortError;
use agent_domain::entities::{Agent, Session};
use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::{
    AgentCommand, DisplayName, ExitResult, MachineId, Objective, OfficeAdditions, TempMark,
};
use kernel::{Actor, Timestamp};
use serde_json::json;

use super::*;

fn agent(id: u128, status: AgentStatus, temp: bool) -> Agent {
    let mut agent = Agent::new(
        AgentId::from_u128(id),
        DisplayName::parse("name", &format!("agent {id}")).expect("name"),
        AgentCommand::parse("claude").expect("command"),
        Some(std::env::temp_dir()),
        None,
        MachineId::generate(),
        Timestamp::from_unix_millis(0),
    )
    .expect("agent");
    agent.status = status;
    if temp {
        agent.temp = Some(TempMark {
            objective: Objective::new("job").expect("objective"),
            creator: Actor::User,
        });
    }
    agent
}

struct FakeAgents(Vec<Agent>);

impl AgentStore for FakeAgents {
    fn insert(&self, _agent: &Agent) -> Result<(), PortError> {
        Ok(())
    }

    fn find(&self, id: AgentId) -> Result<Option<Agent>, PortError> {
        Ok(self.0.iter().find(|agent| agent.id == id).cloned())
    }

    fn list(&self) -> Result<Vec<Agent>, PortError> {
        Ok(self.0.clone())
    }

    fn update_status(&self, _id: AgentId, _status: AgentStatus) -> Result<(), PortError> {
        Ok(())
    }

    fn update_task(
        &self,
        _id: AgentId,
        _task: Option<&agent_domain::value_objects::TaskText>,
    ) -> Result<(), PortError> {
        Ok(())
    }

    fn delete(&self, _id: AgentId) -> Result<(), PortError> {
        Ok(())
    }
}

struct FakeSessions(Vec<Session>);

impl SessionStore for FakeSessions {
    fn insert(&self, _session: &Session) -> Result<(), PortError> {
        Ok(())
    }

    fn find(&self, id: SessionId) -> Result<Option<Session>, PortError> {
        Ok(self.0.iter().find(|session| session.id == id).cloned())
    }

    fn list_for_agent(&self, _agent_id: AgentId) -> Result<Vec<Session>, PortError> {
        Ok(Vec::new())
    }

    fn latest_for_agent(&self, _agent_id: AgentId) -> Result<Option<Session>, PortError> {
        Ok(None)
    }

    fn running(&self) -> Result<Vec<Session>, PortError> {
        Ok(Vec::new())
    }

    fn set_cli_session_id(&self, _id: SessionId, _cli_session_id: &str) -> Result<(), PortError> {
        Ok(())
    }

    fn end_running(
        &self,
        _agent_id: AgentId,
        _result: &ExitResult,
        _at: Timestamp,
    ) -> Result<bool, PortError> {
        Ok(false)
    }
}

fn session_of(agent_id: AgentId) -> Session {
    Session::begin(
        SessionId::generate(),
        agent_id,
        None,
        None,
        OfficeAdditions::default(),
        Timestamp::from_unix_millis(0),
    )
}

fn request(kind: &str, fields: Value) -> Request {
    let Value::Object(fields) = fields else {
        panic!("a request body is an object");
    };
    Request {
        id: 1,
        kind: kind.to_owned(),
        fields,
    }
}

fn endings(agents: Vec<Agent>, sessions: Vec<Session>) -> TempEndings {
    TempEndings::new(
        Arc::new(FakeAgents(agents)),
        Arc::new(FakeSessions(sessions)),
    )
}

#[test]
fn a_temp_joins_the_set_once_and_leaves_it() {
    let endings = endings(Vec::new(), Vec::new());
    let temp = AgentId::from_u128(1);

    assert!(endings.claim(temp));
    assert!(!endings.claim(temp));
    assert!(endings.contains(temp));
    endings.release(temp);

    assert!(!endings.contains(temp));
    assert!(endings.claim(temp));
}

#[test]
fn a_done_sender_that_is_not_a_temp_is_never_claimed_and_can_still_start() {
    let member = agent(1, AgentStatus::WaitingForInput, false);
    let endings = endings(vec![member], Vec::new());
    let member = AgentId::from_u128(1);

    assert_eq!(endings.claim_temp(member), Err(EndOutcome::NotATemp));

    assert!(!endings.contains(member));
    let named = json!({"agent_id": member.to_string()});
    for kind in ["agent.start", "agent.new_task"] {
        assert!(!endings.refuses(&request(kind, named.clone())), "{kind}");
    }
}

#[test]
fn a_temp_is_claimed_once_and_a_gone_agent_never() {
    let endings = endings(
        vec![agent(1, AgentStatus::WaitingForInput, true)],
        Vec::new(),
    );
    let temp = AgentId::from_u128(1);

    assert_eq!(endings.claim_temp(temp), Ok(()));
    assert_eq!(endings.claim_temp(temp), Err(EndOutcome::AlreadyEnding));
    assert_eq!(
        endings.claim_temp(AgentId::from_u128(2)),
        Err(EndOutcome::Gone)
    );
    assert!(!endings.contains(AgentId::from_u128(2)));
}

#[test]
fn a_create_in_progress_shows_until_its_guard_drops() {
    let endings = endings(Vec::new(), Vec::new());
    assert!(!endings.is_creating());

    let guard = endings.begin_create();
    assert!(endings.is_creating());
    drop(guard);

    assert!(!endings.is_creating());
}

#[test]
fn every_request_that_starts_the_temp_again_is_refused_while_it_ends() {
    let temp = AgentId::from_u128(1);
    let session = session_of(temp);
    let session_id = session.id.to_string();
    let endings = endings(Vec::new(), vec![session]);
    assert!(endings.claim(temp));
    let named = json!({"agent_id": temp.to_string()});

    for kind in [
        "agent.start",
        "agent.new_task",
        "agent.resume",
        "agent.wake",
    ] {
        assert!(endings.refuses(&request(kind, named.clone())), "{kind}");
    }
    assert!(endings.refuses(&request(
        "session.continue",
        json!({"session_id": session_id})
    )));
    assert!(!endings.refuses(&request("agent.stop", named.clone())));
    assert!(!endings.refuses(&request("agent.delete", named)));
}

#[test]
fn a_request_for_another_agent_or_with_no_temp_ending_passes() {
    let temp = AgentId::from_u128(1);
    let other = json!({"agent_id": AgentId::from_u128(2).to_string()});
    let endings = endings(Vec::new(), Vec::new());

    assert!(!endings.refuses(&request(
        "agent.start",
        json!({"agent_id": temp.to_string()})
    )));
    assert!(endings.claim(temp));
    assert!(!endings.refuses(&request("agent.start", other)));
    assert!(!endings.refuses(&request("agent.start", json!({"agent_id": "not an id"}))));
}

fn check(endings: &TempEndings, agent_id: AgentId, seen: &str) -> Response {
    endings.end_check(&request(
        END_CHECK_REQUEST,
        json!({"agent_id": agent_id.to_string(), "seen": seen}),
    ))
}

fn ending_of(response: &Response) -> Value {
    match response {
        Response::Success { result, .. } => result["ending"].clone(),
        Response::Failure { message, .. } => panic!("the check failed: {message}"),
    }
}

#[test]
fn the_end_check_claims_a_temp_whose_status_is_the_one_seen_and_only_once() {
    let temp = agent(1, AgentStatus::Stopped, true);
    let endings = endings(vec![temp.clone()], Vec::new());

    assert_eq!(ending_of(&check(&endings, temp.id, "stopped")), json!(true));
    assert!(endings.contains(temp.id));
    assert_eq!(
        ending_of(&check(&endings, temp.id, "stopped")),
        json!(false)
    );
}

#[test]
fn the_end_check_claims_nothing_when_the_status_changed_or_the_agent_is_no_temp() {
    let running = agent(1, AgentStatus::Working, true);
    let plain = agent(2, AgentStatus::Stopped, false);
    let endings = endings(vec![running.clone(), plain.clone()], Vec::new());

    assert_eq!(
        ending_of(&check(&endings, running.id, "stopped")),
        json!(false)
    );
    assert_eq!(
        ending_of(&check(&endings, plain.id, "stopped")),
        json!(false)
    );
    assert_eq!(
        ending_of(&check(&endings, AgentId::from_u128(9), "stopped")),
        json!(false)
    );
    assert!(!endings.contains(running.id) && !endings.contains(plain.id));
}

#[test]
fn an_idle_end_that_read_a_waiting_temp_claims_nothing_once_a_new_task_made_it_work() {
    let working_again = agent(1, AgentStatus::Working, true);
    let endings = endings(vec![working_again.clone()], Vec::new());

    let answer = check(&endings, working_again.id, "waiting_for_input");

    assert_eq!(ending_of(&answer), json!(false));
    assert!(!endings.contains(working_again.id));
}

#[test]
fn the_end_check_names_a_request_without_its_fields_invalid() {
    let endings = endings(Vec::new(), Vec::new());

    let answer = endings.end_check(&request(END_CHECK_REQUEST, json!({})));

    assert!(matches!(
        answer,
        Response::Failure {
            code: ErrorCode::Invalid,
            ..
        }
    ));
}

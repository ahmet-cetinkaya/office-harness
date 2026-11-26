// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

use kernel::AgentId;
use serde_json::{Value, json};

use super::{
    Flow, ResumeResult, SessionEnd, Started, WakeResult, end_of, final_wake_result, hold_reason,
    status_of,
};

fn event_body(result: &ResumeResult, agent_id: AgentId) -> Value {
    serde_json::from_str(&result.event(Flow::Resume, agent_id).encode()).expect("event json")
}

fn started(continued: bool, did_not_start: Option<&str>) -> Started {
    Started {
        session_id: "s1".to_owned(),
        continued,
        reason: None,
        did_not_start: did_not_start.map(str::to_owned),
    }
}

#[test]
fn a_checking_result_is_not_final_and_says_whether_it_continued() {
    let agent_id = AgentId::generate();
    let checking = ResumeResult::Checking {
        continued: true,
        reason: None,
    };

    let body = event_body(&checking, agent_id);

    assert_eq!(body["type"], "resume.result");
    assert_eq!(body["agent_id"], agent_id.to_string());
    assert_eq!(body["result"], "checking");
    assert_eq!(body["continued"], true);
    assert_eq!(body["is_final"], false);
    assert!(body.get("reason").is_none());
}

#[test]
fn final_results_carry_their_reason() {
    let agent_id = AgentId::generate();
    let held = ResumeResult::Held {
        reason: "the resume failed".to_owned(),
    };

    let body = event_body(&held, agent_id);

    assert_eq!(body["result"], "held");
    assert_eq!(body["is_final"], true);
    assert_eq!(body["reason"], "the resume failed");
    assert!(body.get("continued").is_none());
}

#[test]
fn an_answer_with_a_spawn_error_is_a_start_that_did_not_run() {
    let answer = json!({
        "id": "s9",
        "exit_result": "spawn-error: the folder is missing",
        "continued": false,
        "reason": "new session: the CLI cannot continue",
    });

    let started = Started::from_answer(&answer).expect("a session");

    assert_eq!(started.session_id, "s9");
    assert!(!started.continued);
    assert_eq!(
        started.reason.as_deref(),
        Some("new session: the CLI cannot continue")
    );
    assert_eq!(
        started.did_not_start.as_deref(),
        Some("the folder is missing")
    );
}

#[test]
fn an_answer_without_a_session_id_is_not_a_start() {
    assert!(Started::from_answer(&json!({"continued": true})).is_none());
}

#[test]
fn only_exited_and_failed_count_as_an_end_by_itself() {
    let after = Duration::from_secs(2);

    assert_eq!(
        end_of("exited", after),
        Some(SessionEnd::ByItself { after })
    );
    assert_eq!(
        end_of("failed", after),
        Some(SessionEnd::ByItself { after })
    );
    assert_eq!(end_of("stopped", after), Some(SessionEnd::ByOthers));
    assert_eq!(end_of("interrupted", after), Some(SessionEnd::ByOthers));
    assert_eq!(end_of("working", after), None);
    assert_eq!(end_of("starting", after), None);
}

#[test]
fn the_hold_reason_names_how_the_new_session_failed() {
    let ended = SessionEnd::ByItself {
        after: Duration::from_millis(1_900),
    };

    assert_eq!(
        hold_reason(Flow::Resume, &started(false, None), ended),
        "the resume failed: the new session ended 1 s after its start"
    );
    assert_eq!(
        hold_reason(
            Flow::Resume,
            &started(false, Some("no such program")),
            ended
        ),
        "the resume failed: the new session did not start: no such program"
    );
}

#[test]
fn a_status_is_read_only_from_this_agents_status_events() {
    let agent_id = AgentId::generate();
    let other = AgentId::generate();
    let own = json!({"type": "agent.status", "agent_id": agent_id.to_string(), "status": "exited"});
    let foreign =
        json!({"type": "agent.status", "agent_id": other.to_string(), "status": "exited"});
    let line = json!({"type": "agent.line", "agent_id": agent_id.to_string()});

    assert_eq!(
        status_of(&own.to_string(), agent_id).as_deref(),
        Some("exited")
    );
    assert_eq!(status_of(&foreign.to_string(), agent_id), None);
    assert_eq!(status_of(&line.to_string(), agent_id), None);
}

#[test]
fn a_wake_result_has_its_own_event_and_the_hold_names_the_wake() {
    let agent_id = AgentId::from_u128(3);
    let event: Value =
        serde_json::from_str(&ResumeResult::Continued.event(Flow::Wake, agent_id).encode())
            .expect("event json");
    assert_eq!(event["type"], json!("wake.result"));
    assert_eq!(event["result"], json!("continued"));

    let ended = SessionEnd::ByItself {
        after: Duration::from_secs(1),
    };
    assert_eq!(
        hold_reason(Flow::Wake, &started(false, None), ended),
        "the wake failed: the new session ended 1 s after its start"
    );
}

fn wake_frame(agent_id: AgentId, result: &ResumeResult) -> String {
    result.event(Flow::Wake, agent_id).encode()
}

#[test]
fn a_schedule_run_waits_for_the_final_wake_result_of_its_own_agent_only() {
    let agent_id = AgentId::generate();
    let checking = ResumeResult::Checking {
        continued: true,
        reason: None,
    };

    assert_eq!(
        final_wake_result(&wake_frame(agent_id, &checking), agent_id),
        None
    );
    assert_eq!(
        final_wake_result(
            &wake_frame(agent_id, &ResumeResult::Continued),
            AgentId::generate()
        ),
        None
    );
    assert_eq!(
        final_wake_result(
            &ResumeResult::Continued
                .event(Flow::Resume, agent_id)
                .encode(),
            agent_id
        ),
        None,
        "a resume result is no wake result"
    );
}

#[test]
fn the_final_wake_results_map_to_what_a_run_does_next() {
    let agent_id = AgentId::generate();
    let result_of =
        |result: &ResumeResult| final_wake_result(&wake_frame(agent_id, result), agent_id);

    assert_eq!(
        result_of(&ResumeResult::Continued),
        Some(WakeResult::Continued)
    );
    assert_eq!(
        result_of(&ResumeResult::NewSession { reason: None }),
        Some(WakeResult::NewSession)
    );
    assert_eq!(
        result_of(&ResumeResult::Held {
            reason: "the wake failed".to_owned()
        }),
        Some(WakeResult::Held("the wake failed".to_owned()))
    );
    assert_eq!(
        result_of(&ResumeResult::Skipped {
            reason: "no longer asleep".to_owned()
        }),
        Some(WakeResult::Refused("no longer asleep".to_owned()))
    );
}

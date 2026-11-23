// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(unix)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! The counts of the success criteria of 0026: many secrets on disk (SC-001),
//! a rotation across 30 running agents (SC-003), 100 denied reads (SC-004),
//! an export with secrets in the office (SC-008), and the key file custody
//! (SC-006).

mod support;

use std::os::unix::fs::PermissionsExt;

use futures_util::future::join_all;
use serde_json::{Value, json};
use support::office_client::Office;
use support::secret_agents::{
    Caller, call, contains, data_folder_bytes, grant, running_agent, save,
};

fn marker(number: usize) -> String {
    format!("unique-marker-value-{number:04}")
}

#[tokio::test]
async fn twenty_secrets_leave_no_value_in_any_file_of_the_office() {
    let office = Office::open().await;
    let mut client = office.client().await;
    for number in 0..20 {
        save(&mut client, &format!("KEY_{number}"), &marker(number)).await;
    }
    let database = office.serving.office.config.database_path();
    let folder = database.parent().expect("data folder").to_path_buf();
    let _running = office.serving.stop().await;

    let bytes = data_folder_bytes(&folder);

    for number in 0..20 {
        assert!(!contains(&bytes, &marker(number)), "value {number}");
    }
}

#[tokio::test]
async fn a_new_value_reaches_thirty_running_agents_at_their_next_read_without_a_restart() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let mut agents: Vec<(String, Caller)> = Vec::new();
    for number in 0..30 {
        agents.push(running_agent(&office, &mut client, &format!("worker-{number}")).await);
    }
    save(&mut client, "API_KEY", &marker(1)).await;
    for (agent_id, _) in &agents {
        grant(&mut client, "API_KEY", agent_id).await;
    }
    client
        .set_setting("limit_secret_reads_per_agent", "50")
        .await;

    let first = read_everywhere(&agents).await;
    save(&mut client, "API_KEY", &marker(2)).await;
    let second = read_everywhere(&agents).await;

    for answer in &first {
        assert_eq!(answer["result"]["value"], marker(1), "{answer}");
        assert_eq!(answer["result"]["version"], 1);
    }
    for answer in &second {
        assert_eq!(answer["result"]["value"], marker(2), "{answer}");
        assert_eq!(answer["result"]["version"], 2);
    }
    for (agent_id, _) in &agents {
        assert_eq!(client.sessions(agent_id).await.len(), 1, "no restart");
    }
    let listed = client.ask("secret.list", json!({})).await;
    let grants = listed["result"]["secrets"][0]["grants"]
        .as_array()
        .expect("grants");
    assert!(
        grants.iter().all(|held| held["last_read_version"] == 2),
        "{listed}"
    );
    let _running = office.serving.stop().await;
}

/// The most helpers that connect at the same moment: the server lets 8
/// connections be in their handshake at once (0001), and an agent reads when
/// its own work needs the value, not all at one instant.
const READS_AT_ONCE: usize = 6;

async fn read_everywhere(agents: &[(String, Caller)]) -> Vec<Value> {
    let fields = json!({"name": "API_KEY"});
    let mut answers = Vec::new();
    for group in agents.chunks(READS_AT_ONCE) {
        answers.extend(
            join_all(
                group
                    .iter()
                    .map(|(_, caller)| call(caller, "secret.get", &fields)),
            )
            .await,
        );
    }
    answers
}

#[tokio::test]
async fn a_hundred_reads_of_what_an_agent_may_not_read_all_get_one_text_and_no_value() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let (agent_id, caller) = running_agent(&office, &mut client, "worker").await;
    save(&mut client, "OTHER_KEY", &marker(3)).await;
    save(&mut client, "GONE_KEY", &marker(4)).await;
    grant(&mut client, "GONE_KEY", &agent_id).await;
    client
        .ask("secret.delete", json!({"name": "GONE_KEY"}))
        .await;
    client
        .set_setting("limit_secret_reads_per_agent", "1000")
        .await;
    // Ungranted, missing, and deleted names.
    let names = ["OTHER_KEY", "NO_SUCH_KEY", "GONE_KEY"];

    let mut texts = Vec::new();
    for round in 0..100 {
        let name = names[round % names.len()];
        let answer = call(&caller, "secret.get", &json!({"name": name})).await;
        assert_eq!(answer["error"]["code"], "not_found", "{answer}");
        let message = answer["error"]["message"].as_str().expect("message");
        texts.push(message.replace(name, "NAME"));
        assert!(!answer.to_string().contains(&marker(3)));
    }
    let by_user = client.ask("secret.get", json!({"name": "OTHER_KEY"})).await;

    assert!(texts.iter().all(|text| text == &texts[0]), "{texts:?}");
    assert_eq!(by_user["error"]["code"], "not_found");
    let _running = office.serving.stop().await;
}

#[tokio::test]
async fn an_export_of_an_office_with_secrets_holds_no_name_value_or_grant() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let source = client
        .create_agent(&office, "reviewer", "claude", None)
        .await;
    for number in 0..5 {
        save(
            &mut client,
            &format!("SECRET_NAME_{number}"),
            &marker(10 + number),
        )
        .await;
        grant(&mut client, &format!("SECRET_NAME_{number}"), &source).await;
    }
    let copied = client
        .ask("template.from_agent", json!({"agent_id": source}))
        .await;
    let saved = client
        .ask(
            "template.save",
            json!({"name": "Reviewer", "kind": "agent", "body": copied["result"]["body"]}),
        )
        .await;
    assert_eq!(saved["ok"], true, "{saved}");

    let exported = client.ask("template.export", json!({})).await;

    let text = exported["result"]["text"].as_str().expect("text");
    assert!(!text.contains("SECRET_NAME"), "{text}");
    assert!(!text.contains("unique-marker-value"), "{text}");
    assert!(!text.contains(&source), "a grant names the agent id");
    let _running = office.serving.stop().await;
}

#[tokio::test]
async fn without_a_keyring_the_key_goes_to_an_owner_only_file_and_the_list_says_so() {
    let office = Office::open().await;
    let mut client = office.client().await;

    save(&mut client, "API_KEY", &marker(5)).await;

    let listed = client.ask("secret.list", json!({})).await;
    assert_eq!(
        listed["result"]["store"],
        json!({"state": "ready", "custody": "key_file"})
    );
    let key_file = office.serving.office.config.secret_key_path();
    let mode = std::fs::metadata(&key_file)
        .expect("key file")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
    assert_eq!(std::fs::metadata(&key_file).expect("key file").len(), 32);
    let _running = office.serving.stop().await;
}

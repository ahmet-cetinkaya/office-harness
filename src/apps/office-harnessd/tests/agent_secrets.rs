// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(unix)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! Secrets that reach running agents (0026) through the real helper: the
//! person saves and grants, a running agent reads without a restart, a new
//! value reaches it at its next read, and nothing on disk or in a record holds
//! the value.

mod support;

use office_harnessd::bootstrap::disable_os_keyring;
use serde_json::json;
use support::office_client::Office;
use support::secret_agents::{
    FIRST_VALUE, SECOND_VALUE, actions_of, call, contains, data_folder_bytes, grant, helper,
    kind_entries, running_agent, save,
};

#[tokio::test]
async fn a_granted_secret_reaches_a_running_agent_and_a_new_value_needs_no_restart() {
    disable_os_keyring();
    let office = Office::open().await;
    let mut client = office.client().await;
    let (agent_id, caller) = running_agent(&office, &mut client, "worker").await;

    let none = call(&caller, "secret.available", &json!({})).await;
    assert_eq!(none["result"], json!({"secrets": []}), "{none}");
    assert_eq!(save(&mut client, "API_KEY", FIRST_VALUE).await["ok"], true);
    let denied = call(&caller, "secret.get", &json!({"name": "API_KEY"})).await;
    assert_eq!(denied["error"]["code"], "not_found", "{denied}");
    assert_eq!(grant(&mut client, "API_KEY", &agent_id).await["ok"], true);

    let available = call(&caller, "secret.available", &json!({})).await;
    assert_eq!(
        available["result"],
        json!({"secrets": [{"name": "API_KEY", "version": 1}]})
    );
    let first = call(&caller, "secret.get", &json!({"name": "API_KEY"})).await;
    assert_eq!(
        first["result"],
        json!({"name": "API_KEY", "value": FIRST_VALUE, "version": 1}),
        "{first}"
    );
    assert_eq!(save(&mut client, "API_KEY", SECOND_VALUE).await["ok"], true);
    let second = call(&caller, "secret.get", &json!({"name": "API_KEY"})).await;
    assert_eq!(second["result"]["value"], SECOND_VALUE, "{second}");
    assert_eq!(second["result"]["version"], 2);

    assert_eq!(client.sessions(&agent_id).await.len(), 1);
    let listed = client.ask("secret.list", json!({})).await;
    let secret = &listed["result"]["secrets"][0];
    assert_eq!(listed["result"]["store"]["state"], "ready", "{listed}");
    assert_eq!(secret["grants"][0]["last_read_version"], 2, "{secret}");
    assert!(!listed.to_string().contains(SECOND_VALUE));
    office.serving.stop().await;
}

#[tokio::test]
async fn the_helper_gives_the_value_alone_and_nothing_on_an_error() {
    disable_os_keyring();
    let office = Office::open().await;
    let mut client = office.client().await;
    let (agent_id, caller) = running_agent(&office, &mut client, "worker").await;
    save(&mut client, "API_KEY", FIRST_VALUE).await;
    grant(&mut client, "API_KEY", &agent_id).await;
    let fields = json!({"name": "API_KEY"}).to_string();

    let value = helper(
        &caller,
        vec!["secret.get".into(), fields.clone(), "--value-only".into()],
    )
    .await;
    let missing = helper(
        &caller,
        vec![
            "secret.get".into(),
            json!({"name": "NO_SUCH_KEY"}).to_string(),
            "--value-only".into(),
        ],
    )
    .await;

    assert_eq!(String::from_utf8_lossy(&value.stdout), FIRST_VALUE);
    assert!(value.status.success());
    assert_eq!(missing.stdout, b"");
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("no secret named NO_SUCH_KEY"));
    office.serving.stop().await;
}

#[tokio::test]
async fn an_agent_cannot_change_secrets_and_a_user_client_cannot_read_one() {
    disable_os_keyring();
    let office = Office::open().await;
    let mut client = office.client().await;
    let (agent_id, caller) = running_agent(&office, &mut client, "worker").await;
    save(&mut client, "API_KEY", FIRST_VALUE).await;
    grant(&mut client, "API_KEY", &agent_id).await;

    for kind in ["secret.set", "secret.delete", "secret.grant", "secret.list"] {
        let answer = call(&caller, kind, &json!({"name": "API_KEY"})).await;
        assert_eq!(answer["error"]["code"], "refused", "{kind}: {answer}");
    }
    let by_user = client.ask("secret.get", json!({"name": "API_KEY"})).await;

    assert_eq!(by_user["error"]["code"], "not_found", "{by_user}");
    let still = call(&caller, "secret.get", &json!({"name": "API_KEY"})).await;
    assert_eq!(still["result"]["value"], FIRST_VALUE, "{still}");
    office.serving.stop().await;
}

#[tokio::test]
async fn a_read_a_refusal_and_a_change_are_recorded_without_the_value() {
    disable_os_keyring();
    let office = Office::open().await;
    let mut client = office.client().await;
    let (agent_id, caller) = running_agent(&office, &mut client, "worker").await;
    save(&mut client, "API_KEY", FIRST_VALUE).await;
    grant(&mut client, "API_KEY", &agent_id).await;
    call(&caller, "secret.get", &json!({"name": "API_KEY"})).await;
    call(&caller, "secret.get", &json!({"name": "NO_SUCH_KEY"})).await;
    call(
        &caller,
        "secret.get",
        &json!({"name": "API_KEY", "agent_id": "x"}),
    )
    .await;

    let entries = actions_of(&mut client, &agent_id).await;

    let reads = kind_entries(&entries, "secret_read");
    assert_eq!(reads.len(), 3, "{entries:?}");
    assert!(reads.iter().any(|entry| entry["detail"] == "API_KEY v1"));
    assert!(reads.iter().any(|entry| {
        entry["result"]
            .to_string()
            .contains("NO_SUCH_KEY: not available")
    }));
    assert!(
        reads
            .iter()
            .all(|entry| entry["actor"] == format!("agent:{agent_id}"))
    );
    let changes = kind_entries(&entries, "secret_change");
    assert_eq!(changes.len(), 1, "{entries:?}");
    assert_eq!(changes[0]["detail"], "granted API_KEY");
    assert_eq!(changes[0]["actor"], "user");
    assert!(!json!(entries).to_string().contains(FIRST_VALUE));
    office.serving.stop().await;
}

#[tokio::test]
async fn the_reads_limit_and_a_hold_refuse_a_read_and_the_refusal_is_recorded() {
    disable_os_keyring();
    let office = Office::open().await;
    let mut client = office.client().await;
    let (agent_id, caller) = running_agent(&office, &mut client, "worker").await;
    save(&mut client, "API_KEY", FIRST_VALUE).await;
    grant(&mut client, "API_KEY", &agent_id).await;
    client
        .set_setting("limit_secret_reads_per_agent", "2")
        .await;
    let fields = json!({"name": "API_KEY"});

    for _ in 0..2 {
        assert_eq!(call(&caller, "secret.get", &fields).await["ok"], true);
    }
    let limited = call(&caller, "secret.get", &fields).await;
    client.ask("hold.set", json!({"agent_id": agent_id})).await;
    let held = call(&caller, "secret.get", &fields).await;

    assert_eq!(limited["error"]["code"], "refused", "{limited}");
    assert!(
        limited["error"]["message"]
            .as_str()
            .expect("message")
            .contains("reads of secrets by one agent"),
        "{limited}"
    );
    assert_eq!(held["error"]["code"], "refused", "{held}");
    assert!(held.to_string().contains("on hold"), "{held}");
    let entries = actions_of(&mut client, &agent_id).await;
    let failed = kind_entries(&entries, "secret_read")
        .into_iter()
        .filter(|entry| entry["result"] != "ok")
        .count();
    assert_eq!(failed, 2, "{entries:?}");
    office.serving.stop().await;
}

#[tokio::test]
async fn the_clients_hear_of_a_change_by_name_and_the_grants_go_with_a_deleted_agent() {
    disable_os_keyring();
    let office = Office::open().await;
    let mut client = office.client().await;
    let (agent_id, _caller) = running_agent(&office, &mut client, "worker").await;

    let request = client
        .send(
            "secret.set",
            json!({"name": "API_KEY", "value": FIRST_VALUE}),
        )
        .await;
    let event = client.next_event("secret.changed").await;
    assert_eq!(client.answer_of(request).await["ok"], true);
    assert_eq!(event["cause"], "saved", "{event}");
    assert_eq!(event["name"], "API_KEY");
    assert!(!event.to_string().contains(FIRST_VALUE));
    grant(&mut client, "API_KEY", &agent_id).await;
    client
        .ask("agent.stop", json!({"agent_id": agent_id}))
        .await;
    client
        .ask("agent.delete", json!({"agent_id": agent_id}))
        .await;

    let listed = client.ask("secret.list", json!({})).await;

    assert_eq!(
        listed["result"]["secrets"][0]["grants"],
        json!([]),
        "{listed}"
    );
    office.serving.stop().await;
}

#[tokio::test]
async fn no_file_of_the_office_holds_a_value() {
    disable_os_keyring();
    let office = Office::open().await;
    let mut client = office.client().await;
    let (agent_id, caller) = running_agent(&office, &mut client, "worker").await;
    save(&mut client, "API_KEY", FIRST_VALUE).await;
    grant(&mut client, "API_KEY", &agent_id).await;
    call(&caller, "secret.get", &json!({"name": "API_KEY"})).await;
    save(&mut client, "API_KEY", SECOND_VALUE).await;
    call(&caller, "secret.get", &json!({"name": "API_KEY"})).await;
    call(&caller, "secret.get", &json!({"name": "NO_SUCH_KEY"})).await;
    let database = office.serving.office.config.database_path();
    let folder = database.parent().expect("data folder");
    let bytes_before_stop = data_folder_bytes(folder);
    let _running = office.serving.stop().await;

    let bytes = data_folder_bytes(folder);

    for value in [FIRST_VALUE, SECOND_VALUE] {
        assert!(!contains(&bytes_before_stop, value));
        assert!(!contains(&bytes, value));
    }
}

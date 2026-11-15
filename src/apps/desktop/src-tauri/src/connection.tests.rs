// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

const SHORT_LIMIT: Duration = Duration::from_millis(50);

#[tokio::test]
async fn settles_when_the_state_becomes_connected() {
    let (sender, receiver) = watch::channel(ConnectionState::NotReachable);
    let waiting = tokio::spawn(wait_until_settled(receiver, Duration::from_secs(5)));

    sender.send_replace(ConnectionState::Loading);
    sender.send_replace(ConnectionState::Connected);

    assert!(waiting.await.expect("task"));
}

#[tokio::test]
async fn a_state_that_is_already_settled_returns_at_once() {
    let (_sender, receiver) = watch::channel(ConnectionState::NotAuthorized);

    assert!(wait_until_settled(receiver, SHORT_LIMIT).await);
}

#[tokio::test]
async fn gives_up_while_the_server_stays_unreachable() {
    let (_sender, receiver) = watch::channel(ConnectionState::NotReachable);

    assert!(!wait_until_settled(receiver, SHORT_LIMIT).await);
}

#[tokio::test]
async fn a_request_without_a_session_is_not_connected() {
    let connection = Connection::new(DataFolder::at(std::env::temp_dir()));

    let failure = connection
        .request("agent.list".to_owned(), Map::new())
        .await
        .expect_err("no session");

    assert_eq!(failure, ApiError::not_connected());
}

#[tokio::test]
async fn publishing_the_same_state_does_not_wake_watchers() {
    let connection = Connection::new(DataFolder::at(std::env::temp_dir()));
    let mut receiver = connection.subscribe_state();
    receiver.mark_unchanged();

    connection.shared.publish(ConnectionState::Loading);

    assert!(!receiver.has_changed().expect("open"));
}

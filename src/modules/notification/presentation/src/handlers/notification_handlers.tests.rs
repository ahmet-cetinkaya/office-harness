// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use kernel::{AgentId, Timestamp};
use notification_application::commands::read_agent_notifications::ReadAgentNotificationsHandler;
use notification_application::commands::read_notification::ReadNotificationHandler;
use notification_application::common::interfaces::NotificationStore;
use notification_application::queries::list_notifications::LIST_LIMIT;
use notification_application::queries::list_notifications::ListNotificationsHandler;
use notification_domain::{Notification, NotificationEvent, NotificationId};
use serde_json::{Map, Value, json};
use wire::{ErrorCode, Request, Response};

use super::*;
use crate::test_support::{MemoryNotifications, TestClock};

const NOW: Timestamp = Timestamp::from_unix_millis(1_700_000_000_000);

fn request(kind: &str, fields: Value) -> Request {
    let Value::Object(fields) = fields else {
        panic!("an object");
    };
    Request {
        id: 7,
        kind: kind.to_owned(),
        fields: Map::from_iter(fields),
    }
}

fn handlers_with(store: Arc<MemoryNotifications>) -> NotificationHandlers {
    let store: Arc<dyn NotificationStore> = store;
    let clock = Arc::new(TestClock(NOW));
    NotificationHandlers::new(NotificationUseCases {
        list: ListNotificationsHandler::new(Arc::clone(&store)),
        read: ReadNotificationHandler::new(Arc::clone(&store), Arc::clone(&clock) as _),
        read_agent: ReadAgentNotificationsHandler::new(store, clock),
    })
}

fn push(
    store: &MemoryNotifications,
    agent_id: AgentId,
    name: &str,
    event: NotificationEvent,
    created_at: i64,
) -> NotificationId {
    let notification = Notification::new(
        NotificationId::generate(),
        agent_id,
        name.to_owned(),
        event,
        Timestamp::from_unix_millis(created_at),
    );
    let id = notification.id;
    store.push(&notification);
    id
}

#[test]
fn the_list_answers_unread_first_with_every_field_of_the_contract() {
    let store = Arc::new(MemoryNotifications::default());
    let agent = AgentId::generate();
    push(&store, agent, "Ana", NotificationEvent::Exited, 10);
    let read_id = push(&store, agent, "Ana", NotificationEvent::Bell, 20);
    store.with_read(read_id, Timestamp::from_unix_millis(30));
    let handlers = handlers_with(Arc::clone(&store));

    let response = handlers.handle(&request("notification.list", json!({})));

    let Response::Success { id, result } = response else {
        panic!("success");
    };
    assert_eq!(id, 7);
    assert_eq!(result[0]["event"], "exited");
    assert_eq!(result[0]["agent_name"], "Ana");
    assert_eq!(result[0]["read_at"], Value::Null);
    assert_eq!(result[1]["id"], read_id.as_uuid_text());
    assert_eq!(result[1]["read_at"], json!(30));
}

#[test]
fn unread_only_keeps_the_unread_rows() {
    let store = Arc::new(MemoryNotifications::default());
    let agent = AgentId::generate();
    let read_id = push(&store, agent, "Ana", NotificationEvent::Exited, 10);
    store.with_read(read_id, Timestamp::from_unix_millis(30));
    push(&store, agent, "Ana", NotificationEvent::Bell, 20);
    let handlers = handlers_with(Arc::clone(&store));

    let response = handlers.handle(&request("notification.list", json!({"unread_only": true})));

    let Response::Success { result, .. } = response else {
        panic!("success");
    };
    assert_eq!(result.as_array().expect("array").len(), 1);
    assert_eq!(result[0]["event"], "bell");
}

#[test]
fn unread_only_caps_the_rows_like_the_store_contract() {
    let store = Arc::new(MemoryNotifications::default());
    let agent = AgentId::generate();
    for created_at in 0..520 {
        push(&store, agent, "Ana", NotificationEvent::Exited, created_at);
    }
    let handlers = handlers_with(Arc::clone(&store));

    let response = handlers.handle(&request("notification.list", json!({"unread_only": true})));

    let Response::Success { result, .. } = response else {
        panic!("success");
    };
    assert_eq!(result.as_array().expect("array").len(), LIST_LIMIT);
}

#[test]
fn a_read_marks_the_row_and_an_unknown_id_is_not_found() {
    let store = Arc::new(MemoryNotifications::default());
    let stored = push(
        &store,
        AgentId::generate(),
        "Ana",
        NotificationEvent::Exited,
        10,
    );
    let handlers = handlers_with(Arc::clone(&store));

    let response = handlers.handle(&request(
        "notification.read",
        json!({"notification_id": stored.as_uuid_text()}),
    ));
    assert!(matches!(response, Response::Success { .. }));
    let listed = store.list();
    assert_eq!(listed[0].read_at, Some(NOW));

    let missing = handlers.handle(&request(
        "notification.read",
        json!({"notification_id": NotificationId::generate().as_uuid_text()}),
    ));
    let Response::Failure { code, message, .. } = missing else {
        panic!("failure");
    };
    assert_eq!(code, ErrorCode::NotFound);
    assert!(message.contains("was not found"), "{message}");
}

#[test]
fn read_agent_marks_one_agents_rows_and_never_fails_on_an_unknown_agent() {
    let store = Arc::new(MemoryNotifications::default());
    let ana = AgentId::generate();
    let raj = AgentId::generate();
    push(&store, ana, "Ana", NotificationEvent::Exited, 10);
    push(&store, raj, "Raj", NotificationEvent::Failed, 20);
    let handlers = handlers_with(Arc::clone(&store));

    let response = handlers.handle(&request(
        "notification.read_agent",
        json!({"agent_id": ana.to_string()}),
    ));
    assert!(matches!(response, Response::Success { .. }));
    let unread = store.list();
    assert_eq!(unread[0].agent_name, "Raj");
    assert_eq!(unread[0].read_at, None);
    assert_eq!(unread[1].read_at, Some(NOW));

    let unknown = handlers.handle(&request(
        "notification.read_agent",
        json!({"agent_id": AgentId::generate().to_string()}),
    ));
    assert!(matches!(unknown, Response::Success { .. }));
}

#[test]
fn a_bad_body_is_invalid_and_names_the_field_and_an_unknown_type_is_not_found() {
    let handlers = handlers_with(Arc::new(MemoryNotifications::default()));

    let bad_id = handlers.handle(&request("notification.read", json!({"notification_id": 5})));
    let Response::Failure { code, message, .. } = bad_id else {
        panic!("failure");
    };
    assert_eq!(code, ErrorCode::Invalid);
    assert!(message.contains("notification_id"), "{message}");

    let bad_agent = handlers.handle(&request(
        "notification.read_agent",
        json!({"agent_id": "nope"}),
    ));
    let Response::Failure { code, .. } = bad_agent else {
        panic!("failure");
    };
    assert_eq!(code, ErrorCode::Invalid);

    let other = handlers.handle(&request("notification.hide", json!({})));
    let Response::Failure { code, .. } = other else {
        panic!("failure");
    };
    assert_eq!(code, ErrorCode::NotFound);
}

#[test]
fn every_notification_type_is_accepted_by_the_prefix_check() {
    assert!(NotificationHandlers::accepts("notification.list"));
    assert!(!NotificationHandlers::accepts("notification"));
    assert!(!NotificationHandlers::accepts("agent.list"));
}

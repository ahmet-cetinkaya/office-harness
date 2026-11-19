// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use kernel::{AgentId, Timestamp};
use notification_domain::NotificationEvent;

use super::*;
use crate::commands::read_notification::ReadNotificationHandler;
use crate::common::interfaces::NotificationStore;
use crate::test_support::{MemoryNotifications, TestClock};

const NOW: Timestamp = Timestamp::from_unix_millis(1_700_000_000_000);

fn create_handler(store: &Arc<MemoryNotifications>) -> CreateNotificationHandler {
    CreateNotificationHandler::new(Arc::clone(store) as Arc<dyn NotificationStore>)
}

fn read_handler(store: &Arc<MemoryNotifications>) -> ReadNotificationHandler {
    ReadNotificationHandler::new(
        Arc::clone(store) as Arc<dyn NotificationStore>,
        Arc::new(TestClock(NOW)),
    )
}

fn bell(agent_id: AgentId, at: Timestamp) -> CreateNotification {
    CreateNotification {
        agent_id,
        agent_name: "Ana".to_owned(),
        event: NotificationEvent::Bell,
        at,
        detail: None,
    }
}

#[test]
fn a_second_bell_while_the_first_is_unread_adds_nothing() {
    let store = Arc::new(MemoryNotifications::default());
    let create = create_handler(&store);
    let agent = AgentId::generate();

    let first = create.handle(bell(agent, NOW)).expect("first bell");
    let second = create.handle(bell(agent, NOW)).expect("second bell");

    assert!(first.is_some_and(|notification| notification.is_unread()));
    assert_eq!(second, None);
    assert_eq!(store.all().len(), 1);
}

#[test]
fn a_bell_after_the_unread_one_was_read_creates_a_new_one() {
    let store = Arc::new(MemoryNotifications::default());
    let create = create_handler(&store);
    let read = read_handler(&store);
    let agent = AgentId::generate();
    let first = create
        .handle(bell(agent, NOW))
        .expect("first bell")
        .expect("stored");

    read.handle(first.id).expect("read");

    let again = create.handle(bell(agent, NOW)).expect("second round");
    assert!(again.is_some());
    assert_eq!(store.all().len(), 2);
}

#[test]
fn the_bells_of_two_agents_do_not_coalesce() {
    let store = Arc::new(MemoryNotifications::default());
    let create = create_handler(&store);

    create
        .handle(bell(AgentId::generate(), NOW))
        .expect("first agent");
    let other = create
        .handle(bell(AgentId::generate(), NOW))
        .expect("second agent");

    assert!(other.is_some());
    assert_eq!(store.all().len(), 2);
}

#[test]
fn every_non_bell_event_creates_its_own_row() {
    let store = Arc::new(MemoryNotifications::default());
    let create = create_handler(&store);
    let agent = AgentId::generate();

    for event in [NotificationEvent::Exited, NotificationEvent::Exited] {
        create
            .handle(CreateNotification {
                agent_id: agent,
                agent_name: "Ana".to_owned(),
                event,
                at: NOW,
                detail: None,
            })
            .expect("exit row");
    }

    assert_eq!(store.all().len(), 2);
}

// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use kernel::{AgentId, Timestamp};
use notification_domain::{Notification, NotificationEvent};

use super::*;
use crate::commands::read_agent_notifications::ReadAgentNotificationsHandler;
use crate::test_support::{MemoryNotifications, TestClock, notification_of};

const NOW: Timestamp = Timestamp::from_unix_millis(1_700_000_000_000);

fn stored(
    agent_id: AgentId,
    name: &str,
    event: NotificationEvent,
    created_at: i64,
) -> Notification {
    notification_of(agent_id, name, event, created_at)
}

fn read_agent_handler(store: &Arc<MemoryNotifications>) -> ReadAgentNotificationsHandler {
    ReadAgentNotificationsHandler::new(
        Arc::clone(store) as Arc<dyn crate::common::interfaces::NotificationStore>,
        Arc::new(TestClock(NOW)),
    )
}

fn list_handler(store: &Arc<MemoryNotifications>) -> ListNotificationsHandler {
    ListNotificationsHandler::new(
        Arc::clone(store) as Arc<dyn crate::common::interfaces::NotificationStore>
    )
}

fn created_times(notifications: &[Notification]) -> Vec<i64> {
    notifications
        .iter()
        .map(|notification| notification.created_at.as_unix_millis())
        .collect()
}

#[test]
fn listing_is_unread_first_then_newest_first() {
    let store = Arc::new(MemoryNotifications::default());
    let agent = AgentId::generate();
    let mut read_row = stored(agent, "Ana", NotificationEvent::Bell, 30);
    read_row.mark_read(NOW);
    store.seed(stored(agent, "Ana", NotificationEvent::Exited, 10));
    store.seed(stored(agent, "Ana", NotificationEvent::Failed, 20));
    store.seed(read_row);
    let list = list_handler(&store);

    let listed = list.handle(false).expect("list");

    assert_eq!(created_times(&listed), [20, 10, 30]);
}

#[test]
fn unread_only_drops_the_read_rows() {
    let store = Arc::new(MemoryNotifications::default());
    let agent = AgentId::generate();
    let mut read_row = stored(agent, "Ana", NotificationEvent::Exited, 10);
    read_row.mark_read(NOW);
    store.seed(read_row);
    store.seed(stored(agent, "Ana", NotificationEvent::Bell, 20));
    let list = list_handler(&store);

    let listed = list.handle(true).expect("list unread only");

    assert_eq!(created_times(&listed), [20]);
}

#[test]
fn the_list_stops_at_500_rows_with_the_newest_kept() {
    let store = Arc::new(MemoryNotifications::default());
    let agent = AgentId::generate();
    for created_at in 0..520 {
        store.seed(stored(agent, "Ana", NotificationEvent::Exited, created_at));
    }
    let list = list_handler(&store);

    let listed = list.handle(false).expect("list");

    assert_eq!(listed.len(), LIST_LIMIT);
    assert_eq!(created_times(&listed)[0], 519);
    assert_eq!(created_times(&listed)[499], 20);
}

#[test]
fn read_agent_marks_only_that_agents_rows() {
    let store = Arc::new(MemoryNotifications::default());
    let ana = AgentId::generate();
    let raj = AgentId::generate();
    store.seed(stored(ana, "Ana", NotificationEvent::Exited, 10));
    store.seed(stored(ana, "Ana", NotificationEvent::Bell, 20));
    store.seed(stored(raj, "Raj", NotificationEvent::Failed, 30));
    let read_agent = read_agent_handler(&store);
    let list = list_handler(&store);

    let marked = read_agent.handle(ana).expect("read agent");

    assert_eq!(marked, 2);
    let unread_agents: Vec<AgentId> = list
        .handle(true)
        .expect("unread list")
        .iter()
        .map(|notification| notification.agent_id)
        .collect();
    assert_eq!(unread_agents, [raj]);
}

#[test]
fn reading_an_unknown_agent_marks_nothing_and_fails_not() {
    let store = Arc::new(MemoryNotifications::default());
    let read_agent = read_agent_handler(&store);

    let marked = read_agent
        .handle(AgentId::generate())
        .expect("no error for an unknown agent");

    assert_eq!(marked, 0);
    assert_eq!(store.all().len(), 0);
}

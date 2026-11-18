// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use kernel::{AgentId, Timestamp};
use notification_domain::{Notification, NotificationEvent, NotificationId};

use super::*;
use crate::common::errors::NotificationApplicationError;
use crate::common::interfaces::NotificationStore;
use crate::test_support::{MemoryNotifications, TestClock, notification_of};

const NOW: Timestamp = Timestamp::from_unix_millis(1_700_000_000_000);

fn stored_exited() -> Notification {
    notification_of(AgentId::generate(), "Ana", NotificationEvent::Exited, 10)
}

fn handler(store: &Arc<MemoryNotifications>) -> ReadNotificationHandler {
    ReadNotificationHandler::new(
        Arc::clone(store) as Arc<dyn NotificationStore>,
        Arc::new(TestClock(NOW)),
    )
}

#[test]
fn an_unknown_id_is_not_found() {
    let read = handler(&Arc::new(MemoryNotifications::default()));

    let outcome = read.handle(NotificationId::generate());

    assert!(matches!(
        outcome,
        Err(NotificationApplicationError::NotFound(_))
    ));
}

#[test]
fn a_read_row_keeps_its_first_read_time() {
    let store = Arc::new(MemoryNotifications::default());
    let read = handler(&store);
    let stored = store.insert(&stored_exited()).expect("insert");

    read.handle(stored.id).expect("first read");

    let listed = store.list().expect("list");
    assert_eq!(listed[0].read_at, Some(NOW));
}

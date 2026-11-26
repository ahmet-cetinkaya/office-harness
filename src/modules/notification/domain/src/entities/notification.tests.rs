// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::{AgentId, Timestamp};

use super::*;
use crate::enums::NotificationEvent;
use crate::value_objects::NotificationId;

fn unread(event: NotificationEvent, created_at: i64) -> Notification {
    Notification::new(
        NotificationId::generate(),
        Some(AgentId::generate()),
        "Ana".to_owned(),
        event,
        Timestamp::from_unix_millis(created_at),
    )
}

#[test]
fn a_new_notification_is_unread_and_reading_stamps_it_once() {
    let mut notification = unread(NotificationEvent::Exited, 10);

    assert!(notification.is_unread());
    notification.mark_read(Timestamp::from_unix_millis(20));
    assert_eq!(notification.read_at, Some(Timestamp::from_unix_millis(20)));
    // A second read leaves the first stamp, so the listing order is stable.
    notification.mark_read(Timestamp::from_unix_millis(30));
    assert_eq!(notification.read_at, Some(Timestamp::from_unix_millis(20)));
}

#[test]
fn the_wire_names_of_the_four_events_round_trip() {
    for event in NotificationEvent::ALL {
        let parsed: NotificationEvent = event.as_str().parse().expect("event text");
        assert_eq!(parsed, event);
    }
    assert!("urgent".parse::<NotificationEvent>().is_err());
}

#[test]
fn the_schedule_event_has_its_own_wire_name() {
    assert_eq!(NotificationEvent::Schedule.as_str(), "schedule");
    assert_eq!(
        "schedule".parse::<NotificationEvent>(),
        Ok(NotificationEvent::Schedule)
    );
}

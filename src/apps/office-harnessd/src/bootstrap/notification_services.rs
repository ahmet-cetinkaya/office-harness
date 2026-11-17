// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use notification_application::commands::create_notification::CreateNotificationHandler;
use notification_application::commands::read_agent_notifications::ReadAgentNotificationsHandler;
use notification_application::commands::read_notification::ReadNotificationHandler;
use notification_application::common::interfaces::{NotificationClock, NotificationStore};
use notification_application::queries::list_notifications::ListNotificationsHandler;
use notification_infrastructure::persistence::{NotificationRepository, SharedHandle};
use notification_presentation::{NotificationHandlers, NotificationUseCases};

use crate::bridges::SystemClock;

/// The notification module, wired: the store behind the use cases, and the
/// use cases behind the handlers.
pub struct NotificationServices {
    pub handlers: NotificationHandlers,
    /// Creates a row for the bridge, so the bridge holds no handlers.
    pub create: CreateNotificationHandler,
}

/// Builds the notification services over the shared connection that the host
/// app opened on the office database.
#[must_use]
pub fn build_notification_services(connection: SharedHandle) -> NotificationServices {
    let store: Arc<dyn NotificationStore> = Arc::new(NotificationRepository::new(connection));
    let clock: Arc<dyn NotificationClock> = Arc::new(SystemClock);
    let create = CreateNotificationHandler::new(Arc::clone(&store));
    NotificationServices {
        handlers: NotificationHandlers::new(NotificationUseCases {
            list: ListNotificationsHandler::new(Arc::clone(&store)),
            read: ReadNotificationHandler::new(Arc::clone(&store), Arc::clone(&clock)),
            read_agent: ReadAgentNotificationsHandler::new(store, clock),
        }),
        create,
    }
}

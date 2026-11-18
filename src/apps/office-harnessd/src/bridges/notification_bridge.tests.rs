// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use agent_application::common::errors::PortError;
use agent_application::common::interfaces::AgentStore;
use agent_domain::entities::Agent;
use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::{AgentCommand, DisplayName, MachineId};
use kernel::{AgentId, Timestamp};
use notification_application::commands::create_notification::CreateNotificationHandler;
use notification_application::common::errors::NotificationApplicationError;
use notification_application::common::interfaces::NotificationStore;
use notification_application::queries::list_notifications::LIST_LIMIT;
use notification_domain::{Notification, NotificationId};
use terminal_application::common::errors::TerminalRuntimeError;
use terminal_application::common::interfaces::{OutputSubscriber, TerminalRuntime};
use terminal_application::common::models::{
    Attachment, RunId, SpawnTerminalRequest, SubscriptionId,
};
use terminal_domain::value_objects::{OutputKey, TerminalSize};

use super::*;

const NOW: Timestamp = Timestamp::from_unix_millis(1_700_000_000_000);

/// The in-memory store the bridge writes to in the tests, ordered like the
/// listing: unread first, then newest first.
#[derive(Default)]
struct MemoryNotifications(std::sync::Mutex<Vec<Notification>>);

impl MemoryNotifications {
    fn all(&self) -> Vec<Notification> {
        self.0.lock().expect("lock").clone()
    }
}

impl NotificationStore for MemoryNotifications {
    fn insert(
        &self,
        notification: &Notification,
    ) -> Result<Notification, NotificationApplicationError> {
        self.0.lock().expect("lock").push(notification.clone());
        Ok(notification.clone())
    }

    fn list(&self) -> Result<Vec<Notification>, NotificationApplicationError> {
        let mut rows = self.0.lock().expect("lock").clone();
        rows.sort_by(|left, right| {
            let left_unread = u8::from(left.is_unread());
            let right_unread = u8::from(right.is_unread());
            right_unread
                .cmp(&left_unread)
                .then_with(|| right.created_at.cmp(&left.created_at))
        });
        rows.truncate(LIST_LIMIT);
        Ok(rows)
    }

    fn list_unread(&self) -> Result<Vec<Notification>, NotificationApplicationError> {
        let mut rows: Vec<Notification> = self
            .0
            .lock()
            .expect("lock")
            .iter()
            .filter(|row| row.is_unread())
            .cloned()
            .collect();
        rows.sort_by(|left, right| {
            let left_unread = u8::from(left.is_unread());
            let right_unread = u8::from(right.is_unread());
            right_unread
                .cmp(&left_unread)
                .then_with(|| right.created_at.cmp(&left.created_at))
        });
        Ok(rows)
    }

    fn find(
        &self,
        notification_id: NotificationId,
    ) -> Result<Option<Notification>, NotificationApplicationError> {
        Ok(self
            .0
            .lock()
            .expect("lock")
            .iter()
            .find(|row| row.id == notification_id)
            .cloned())
    }

    fn update(&self, notification: &Notification) -> Result<(), NotificationApplicationError> {
        let mut rows = self.0.lock().expect("lock");
        if let Some(row) = rows.iter_mut().find(|row| row.id == notification.id) {
            *row = notification.clone();
        }
        Ok(())
    }

    fn list_unread_for_agent(
        &self,
        agent_id: AgentId,
    ) -> Result<Vec<Notification>, NotificationApplicationError> {
        Ok(self
            .0
            .lock()
            .expect("lock")
            .iter()
            .filter(|row| row.agent_id == agent_id && row.is_unread())
            .cloned()
            .collect())
    }
}

/// The stamp every notification in the tests is created with.
struct FixedClock(Timestamp);

impl agent_application::common::interfaces::Clock for FixedClock {
    fn now(&self) -> Timestamp {
        self.0
    }
}

/// An office with one named agent whose row the bridge can copy, or none
/// once the deletion flag is set.
struct OneAgent {
    agent: Agent,
    deleted: AtomicBool,
}

impl OneAgent {
    fn never_deleted(agent: &Agent) -> Self {
        Self {
            agent: agent.clone(),
            deleted: AtomicBool::new(false),
        }
    }
}

impl AgentStore for OneAgent {
    fn insert(&self, _agent: &Agent) -> Result<(), PortError> {
        Ok(())
    }

    fn find(&self, _id: AgentId) -> Result<Option<Agent>, PortError> {
        if self.deleted.load(Ordering::SeqCst) {
            Ok(None)
        } else {
            Ok(Some(self.agent.clone()))
        }
    }

    fn list(&self) -> Result<Vec<Agent>, PortError> {
        Ok(Vec::new())
    }

    fn update_status(&self, _id: AgentId, _status: AgentStatus) -> Result<(), PortError> {
        Ok(())
    }

    fn delete(&self, _id: AgentId) -> Result<(), PortError> {
        Ok(())
    }
}

/// A runtime that answers one current run, so a late bell of an earlier run
/// can be told apart.
struct RunOnly(Option<RunId>);

impl TerminalRuntime for RunOnly {
    fn spawn(&self, _request: &SpawnTerminalRequest) -> Result<(), TerminalRuntimeError> {
        Ok(())
    }

    fn attach(
        &self,
        agent_id: AgentId,
        _saved_output: Option<&OutputKey>,
        _size: TerminalSize,
        _subscriber: Arc<dyn OutputSubscriber>,
    ) -> Result<Attachment, TerminalRuntimeError> {
        Err(TerminalRuntimeError::NoOutput(agent_id))
    }

    fn detach(&self, _agent_id: AgentId, _subscription: SubscriptionId) {}

    fn write_input(&self, agent_id: AgentId, _bytes: &[u8]) -> Result<(), TerminalRuntimeError> {
        Err(TerminalRuntimeError::NotRunning(agent_id))
    }

    fn paste(&self, agent_id: AgentId, _text: &str) -> Result<(), TerminalRuntimeError> {
        Err(TerminalRuntimeError::NotRunning(agent_id))
    }

    fn resize(
        &self,
        _agent_id: AgentId,
        _size: TerminalSize,
    ) -> Result<bool, TerminalRuntimeError> {
        Ok(false)
    }

    fn stop(&self, _agent_id: AgentId, _grace: Duration) -> Result<(), TerminalRuntimeError> {
        Ok(())
    }

    fn is_running(&self, _agent_id: AgentId) -> bool {
        self.0.is_some()
    }

    fn current_run(&self, _agent_id: AgentId) -> Option<RunId> {
        self.0
    }
}

fn agent_named(name: &str) -> Agent {
    Agent::new(
        AgentId::generate(),
        DisplayName::parse("agent name", name).expect("name"),
        AgentCommand::parse("claude").expect("command"),
        Some(std::env::temp_dir()),
        None,
        MachineId::generate(),
        NOW,
    )
    .expect("agent")
}

fn bridge_for(
    agent: &Agent,
    current_run: Option<RunId>,
) -> (
    NotificationBridge,
    Arc<MemoryNotifications>,
    Arc<OneAgent>,
    EventHub,
) {
    let store: Arc<MemoryNotifications> = Arc::new(MemoryNotifications::default());
    let log_folder = tempfile::tempdir().expect("folder");
    let config = crate::config::Config {
        data_folder: log_folder.path().join("office"),
    };
    let log = crate::bootstrap::open_log(&config).expect("log");
    let hub = EventHub::new();
    let agents = Arc::new(OneAgent::never_deleted(agent));
    let bridge = NotificationBridge {
        create: CreateNotificationHandler::new(Arc::clone(&store) as Arc<dyn NotificationStore>),
        agents: Arc::clone(&agents) as Arc<dyn AgentStore>,
        runtime: Arc::new(RunOnly(current_run)),
        clock: Arc::new(FixedClock(NOW)),
        hub: hub.clone(),
        alerts: StorageAlerts::new(log.clone()),
        log,
    };
    (bridge, store, agents, hub)
}

/// The status frame exactly as the office publishes it.
fn status_frame(agent_id: AgentId, status: AgentStatus) -> String {
    agent_presentation::agent_status_event(agent_id, status, None).encode()
}

fn frame_of(status: &str, agent: &Agent) -> String {
    let status = match status {
        "exited" => AgentStatus::Exited,
        "failed" => AgentStatus::Failed,
        "waiting_for_input" => AgentStatus::WaitingForInput,
        "stopped" => AgentStatus::Stopped,
        "interrupted" => AgentStatus::Interrupted,
        other => panic!("unknown test status {other}"),
    };
    status_frame(agent.id, status)
}

fn created_events(store: &MemoryNotifications) -> Vec<(String, String)> {
    store
        .all()
        .iter()
        .map(|notification| {
            (
                notification.agent_name.clone(),
                notification.event.as_str().to_owned(),
            )
        })
        .collect()
}

/// The event frames published since `hub.subscribe()` was called.
fn published_events(receiver: &mut tokio::sync::broadcast::Receiver<Arc<str>>) -> Vec<String> {
    std::iter::from_fn(|| receiver.try_recv().ok())
        .map(|frame| frame.to_string())
        .collect()
}

mod frames {
    use super::*;

    #[test]
    fn the_four_notifying_statuses_are_read_from_a_frame() {
        let agent = agent_named("Ana");
        for (status, expected) in [
            ("exited", NotificationEvent::Exited),
            ("failed", NotificationEvent::Failed),
            ("waiting_for_input", NotificationEvent::WaitingForInput),
        ] {
            let (agent_id, event) =
                notification_of_frame(&frame_of(status, &agent)).expect("frame");
            assert_eq!(agent_id, agent.id);
            assert_eq!(event, expected);
        }
    }

    #[test]
    fn stopped_and_interrupted_and_other_types_record_nothing() {
        let agent = agent_named("Ana");
        assert_eq!(notification_of_frame(&frame_of("stopped", &agent)), None);
        assert_eq!(
            notification_of_frame(&frame_of("interrupted", &agent)),
            None
        );
        assert_eq!(
            notification_of_frame(&status_frame(agent.id, AgentStatus::Working)),
            None
        );
        let other_type = agent_presentation::agent_line_event(agent.id, "hi").encode();
        assert_eq!(notification_of_frame(&other_type), None);
        assert_eq!(notification_of_frame("not json"), None);
    }
}

mod records {
    use super::*;

    #[test]
    fn each_notifying_status_creates_one_row_with_the_copied_name() {
        for status in ["exited", "failed", "waiting_for_input"] {
            let agent = agent_named("Ana");
            let (bridge, store, _agents, hub) = bridge_for(&agent, Some(RunId(1)));
            let mut published = hub.subscribe();

            bridge.apply_frame(&frame_of(status, &agent));

            assert_eq!(
                created_events(&store),
                vec![("Ana".to_owned(), status.to_owned())]
            );
            let sent = published_events(&mut published);
            assert_eq!(sent.len(), 1, "one event for {status}");
            assert!(sent[0].contains("notification.created"), "{}", sent[0]);
            assert!(sent[0].contains("\"agent_name\":\"Ana\""), "{}", sent[0]);
        }
    }

    #[test]
    fn stopped_and_interrupted_create_nothing() {
        for status in ["stopped", "interrupted"] {
            let agent = agent_named("Ana");
            let (bridge, store, _agents, hub) = bridge_for(&agent, Some(RunId(1)));
            let mut published = hub.subscribe();

            bridge.apply_frame(&frame_of(status, &agent));

            assert_eq!(created_events(&store), [], "{status}");
            assert_eq!(published_events(&mut published), Vec::<String>::new());
        }
    }

    #[test]
    fn a_bell_of_the_current_run_creates_the_notification() {
        let agent = agent_named("Ana");
        let (bridge, store, _agents, _hub) = bridge_for(&agent, Some(RunId(3)));

        bridge.apply_bell(AgentTerminalEvent {
            agent_id: agent.id,
            run: RunId(3),
            event: TerminalEvent::Bell,
        });

        assert_eq!(
            created_events(&store),
            vec![("Ana".to_owned(), "bell".to_owned())]
        );
    }

    #[test]
    fn a_late_bell_of_an_earlier_run_creates_nothing() {
        let agent = agent_named("Ana");
        let (bridge, store, _agents, _hub) = bridge_for(&agent, Some(RunId(3)));

        bridge.apply_bell(AgentTerminalEvent {
            agent_id: agent.id,
            run: RunId(2),
            event: TerminalEvent::Bell,
        });

        assert_eq!(created_events(&store), []);
    }

    #[test]
    fn a_notification_outlives_a_deleted_agent_with_its_copied_name() {
        let agent = agent_named("Ana");
        let (bridge, store, agents, _hub) = bridge_for(&agent, Some(RunId(1)));

        // The exit is recorded while the agent exists; the deletion lands
        // right after. The row keeps the name it copied.
        bridge.apply_frame(&frame_of("exited", &agent));
        agents.deleted.store(true, Ordering::SeqCst);
        assert!(agents.find(agent.id).expect("find").is_none());

        let listed = store.list().expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].agent_name, "Ana");
    }

    #[test]
    fn a_notification_of_an_already_deleted_agent_says_it_was_deleted() {
        let agent = agent_named("Ana");
        let (bridge, store, agents, _hub) = bridge_for(&agent, Some(RunId(1)));
        agents.deleted.store(true, Ordering::SeqCst);

        bridge.apply_frame(&frame_of("exited", &agent));

        let listed = store.list().expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].agent_name, DELETED_AGENT_NAME);
    }

    #[test]
    fn a_second_bell_while_the_first_is_unread_is_coalesced() {
        let agent = agent_named("Ana");
        let (bridge, store, _agents, hub) = bridge_for(&agent, Some(RunId(1)));
        let mut published = hub.subscribe();

        bridge.apply_bell(AgentTerminalEvent {
            agent_id: agent.id,
            run: RunId(1),
            event: TerminalEvent::Bell,
        });
        bridge.apply_bell(AgentTerminalEvent {
            agent_id: agent.id,
            run: RunId(1),
            event: TerminalEvent::Bell,
        });

        assert_eq!(store.all().len(), 1);
        assert_eq!(published_events(&mut published).len(), 1);
    }
}

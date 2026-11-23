// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use agent_application::common::errors::PortError;
use agent_application::common::interfaces::AgentStore;
use agent_domain::entities::Agent;
use agent_domain::enums::{AgentStatus, StatusEvent};
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

fn unread_first_then_newest(rows: &mut [Notification]) {
    rows.sort_by(|left, right| {
        let left_unread = u8::from(left.is_unread());
        let right_unread = u8::from(right.is_unread());
        right_unread
            .cmp(&left_unread)
            .then_with(|| right.created_at.cmp(&left.created_at))
    });
}

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
        unread_first_then_newest(&mut rows);
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
        unread_first_then_newest(&mut rows);
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
            .filter(|row| row.agent_id == Some(agent_id) && row.is_unread())
            .cloned()
            .collect())
    }
}

/// An agent store whose every lookup fails: the disk problem a broken store
/// produces, to tell apart from a real delete.
struct BrokenStore;

impl AgentStore for BrokenStore {
    fn insert(&self, _agent: &Agent) -> Result<(), PortError> {
        Ok(())
    }

    fn find(&self, _id: AgentId) -> Result<Option<Agent>, PortError> {
        Err(PortError::Storage {
            message: "the agent table cannot be read".to_owned(),
            is_disk_failure: true,
        })
    }

    fn list(&self) -> Result<Vec<Agent>, PortError> {
        Err(PortError::Storage {
            message: "the agent table cannot be read".to_owned(),
            is_disk_failure: true,
        })
    }

    fn update_status(&self, _id: AgentId, _status: AgentStatus) -> Result<(), PortError> {
        Ok(())
    }

    fn update_task(
        &self,
        _id: AgentId,
        _task: Option<&agent_domain::value_objects::TaskText>,
    ) -> Result<(), PortError> {
        Ok(())
    }

    fn delete(&self, _id: AgentId) -> Result<(), PortError> {
        Ok(())
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

    fn update_task(
        &self,
        _id: AgentId,
        _task: Option<&agent_domain::value_objects::TaskText>,
    ) -> Result<(), PortError> {
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

    fn quiet_for(&self, _agent_id: AgentId) -> Option<Duration> {
        None
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
    let agents = Arc::new(OneAgent::never_deleted(agent));
    let (bridge, store, hub, _alerts) =
        bridge_over(Arc::clone(&agents) as Arc<dyn AgentStore>, current_run);
    (bridge, store, agents, hub)
}

/// A bridge over any agent store, with the alerts feed, so a test can fail
/// the lookups or watch the surfaced losses.
fn bridge_over(
    agents: Arc<dyn AgentStore>,
    current_run: Option<RunId>,
) -> (
    NotificationBridge,
    Arc<MemoryNotifications>,
    EventHub,
    StorageAlerts,
) {
    let store: Arc<MemoryNotifications> = Arc::new(MemoryNotifications::default());
    let log_folder = tempfile::tempdir().expect("folder");
    let config = crate::config::Config {
        data_folder: log_folder.path().join("office"),
    };
    let log = crate::bootstrap::open_log(&config).expect("log");
    let hub = EventHub::new();
    let alerts = StorageAlerts::new(log.clone());
    let needs_you = NeedsYouBridge::in_memory(
        Arc::clone(&agents),
        hub.clone(),
        alerts.clone(),
        log.clone(),
    );
    let bridge = NotificationBridge {
        create: CreateNotificationHandler::new(Arc::clone(&store) as Arc<dyn NotificationStore>),
        agents,
        runtime: Arc::new(RunOnly(current_run)),
        clock: Arc::new(FixedClock(NOW)),
        quiet_period_ms: QUIET_PERIOD_MS,
        needs_you,
        waits: crate::bridges::WaitTracker::default(),
        waiting_agents: Arc::default(),
        missed_notice: Arc::default(),
        hub: hub.clone(),
        alerts: alerts.clone(),
        log,
    };
    (bridge, store, hub, alerts)
}

const QUIET_PERIOD_MS: u64 = 5_000;

/// The status frame exactly as the office publishes it.
fn status_frame(agent_id: AgentId, status: AgentStatus) -> String {
    agent_presentation::agent_status_event(agent_id, status, None).encode()
}

/// The status frame of a command that ended with `exit_code`, with the text
/// that the status bridge writes for it.
fn frame_of_exit(agent: &Agent, exit_code: u32) -> String {
    let (event, detail) =
        super::super::agent_status_bridge::status_event(TerminalEvent::Exited { exit_code })
            .expect("an exit is a status event");
    let status = match event {
        StatusEvent::ExitedWithCodeZero => AgentStatus::Exited,
        _ => AgentStatus::Failed,
    };
    agent_presentation::agent_status_event(agent.id, status, detail.as_deref()).encode()
}

fn bell_of(agent: &Agent) -> AgentTerminalEvent {
    AgentTerminalEvent {
        agent_id: agent.id,
        run: RunId(1),
        event: TerminalEvent::Bell,
    }
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

/// The (agent, event) that one published status frame asks to record, if any.
fn notification_of_frame(frame: &str) -> Option<(AgentId, NotificationEvent)> {
    let status = StatusFrame::parse(frame)?;
    Some((status.agent_id, status.notification_event()?))
}

/// The `notification.created` frames among the published ones: the 0003
/// tests count these, and the queue adds `needs_you.changed` frames of its own.
fn created_frames(frames: Vec<String>) -> Vec<String> {
    frames
        .into_iter()
        .filter(|frame| frame.contains("notification.created"))
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
            let sent = created_frames(published_events(&mut published));
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
    fn a_failed_name_lookup_records_nothing_and_alerts_instead_of_naming_a_delete() {
        let agent = agent_named("Ana");
        let (bridge, store, _hub, alerts) = bridge_over(Arc::new(BrokenStore), Some(RunId(1)));
        let mut alert_feed = alerts.subscribe();

        bridge.apply_frame(&frame_of("exited", &agent));

        assert_eq!(
            store.all(),
            [],
            "a storage failure must not write a row that names a delete"
        );
        let alert = alert_feed.try_recv().expect("the loss is surfaced");
        assert!(
            alert.message.contains("the agent table cannot be read"),
            "{}",
            alert.message
        );
    }

    #[test]
    fn a_lagged_status_feed_is_surfaced_as_a_loss_the_client_can_resync() {
        let agent = agent_named("Ana");
        let (bridge, store, _hub, alerts) =
            bridge_over(Arc::new(OneAgent::never_deleted(&agent)), Some(RunId(1)));
        let mut alert_feed = alerts.subscribe();

        bridge.missed(3, "status events");

        assert_eq!(store.all(), [], "a lag records no row of its own");
        let alert = alert_feed.try_recv().expect("the lag is surfaced");
        assert_eq!(
            alert.message,
            "the notification bridge missed 3 status events; \
             notifications are missing for them; \
             read notification.list and needs_you.list again"
        );
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
        assert_eq!(created_frames(published_events(&mut published)).len(), 1);
    }
}

#[test]
fn an_auto_hold_row_carries_the_limit_name_as_its_detail_and_reaches_the_clients() {
    let agent = agent_named("Ana");
    let (bridge, store, _agents, hub) = bridge_for(&agent, None);
    let mut frames = hub.subscribe();

    bridge.record_auto_hold(agent.id, "directives sent by one agent");

    let rows = store.all();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].event, NotificationEvent::AutoHold);
    assert_eq!(
        rows[0].detail.as_deref(),
        Some("directives sent by one agent")
    );
    assert!(rows[0].is_unread());
    assert!(published_events(&mut frames)[0].contains("auto_hold"));
}

#[test]
fn a_resume_hold_row_carries_the_reason_as_its_detail_and_reaches_the_clients() {
    let agent = agent_named("Ana");
    let (bridge, store, _agents, hub) = bridge_for(&agent, None);
    let mut frames = hub.subscribe();

    bridge.record_resume_hold(agent.id, "the new session ended 2 s after it started");

    let rows = store.all();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].event, NotificationEvent::ResumeHold);
    assert_eq!(
        rows[0].detail.as_deref(),
        Some("the new session ended 2 s after it started")
    );
    assert!(published_events(&mut frames)[0].contains("resume_hold"));
}

const MINUTE_MILLIS: i64 = 60_000;

#[test]
fn a_schedule_result_row_names_the_schedule_and_the_target_and_adds_no_needs_you_item() {
    let agent = agent_named("Ana");
    let (bridge, store, _agents, hub) = bridge_for(&agent, None);
    let mut frames = hub.subscribe();

    bridge.record_schedule_result("nightly", agent.id, "Ana: skipped: stopped".to_owned());

    let rows = store.all();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].event, NotificationEvent::Schedule);
    assert_eq!(rows[0].agent_id, Some(agent.id));
    assert_eq!(rows[0].agent_name, "nightly");
    assert_eq!(rows[0].detail.as_deref(), Some("Ana: skipped: stopped"));
    assert!(published_events(&mut frames)[0].contains("\"schedule\""));
}

#[test]
fn the_missed_runs_of_one_minute_share_one_row_that_counts_their_schedules() {
    let agent = agent_named("Ana");
    let (bridge, store, _agents, _hub) = bridge_for(&agent, None);
    let minute = Timestamp::from_unix_millis(10 * MINUTE_MILLIS + 5_000);
    let later_in_the_minute = Timestamp::from_unix_millis(10 * MINUTE_MILLIS + 50_000);

    bridge.record_missed_schedules(2, minute);
    bridge.record_missed_schedules(1, later_in_the_minute);

    let rows = store.all();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].agent_id, None);
    assert_eq!(rows[0].agent_name, "3 schedules missed runs");
    assert_eq!(rows[0].event, NotificationEvent::Schedule);
}

#[test]
fn the_missed_runs_of_the_next_minute_get_a_row_of_their_own() {
    let agent = agent_named("Ana");
    let (bridge, store, _agents, _hub) = bridge_for(&agent, None);

    bridge.record_missed_schedules(1, Timestamp::from_unix_millis(10 * MINUTE_MILLIS));
    bridge.record_missed_schedules(1, Timestamp::from_unix_millis(11 * MINUTE_MILLIS));

    let names: Vec<String> = store.all().into_iter().map(|row| row.agent_name).collect();
    assert_eq!(names, ["1 schedule missed runs", "1 schedule missed runs"]);
}

#[test]
fn a_leftover_kept_row_carries_the_os_reason_as_its_detail_and_reaches_the_clients() {
    let agent = agent_named("Ana");
    let (bridge, store, _agents, hub) = bridge_for(&agent, None);
    let mut frames = hub.subscribe();

    bridge.record_leftover_kept(agent.id, "Operation not permitted (os error 1)");

    let rows = store.all();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].event, NotificationEvent::LeftoverKept);
    assert_eq!(
        rows[0].detail.as_deref(),
        Some("Operation not permitted (os error 1)")
    );
    assert!(published_events(&mut frames)[0].contains("leftover_kept"));
}

mod needs_you_queue {
    use super::*;

    fn kinds_and_details(bridge: &NotificationBridge) -> Vec<(String, Option<String>)> {
        bridge
            .needs_you
            .items()
            .iter()
            .map(|item| (item.latest.kind.to_string(), item.latest.detail.clone()))
            .collect()
    }

    #[test]
    fn a_coalesced_bell_updates_the_item_and_creates_no_second_notification() {
        let agent = agent_named("Ana");
        let (bridge, store, _agents, hub) = bridge_for(&agent, Some(RunId(1)));
        bridge.apply_bell(bell_of(&agent));
        let first = bridge.needs_you.items().remove(0);
        let mut published = hub.subscribe();

        bridge.apply_bell(bell_of(&agent));

        assert_eq!(store.all().len(), 1);
        let item = bridge.needs_you.items().remove(0);
        assert_eq!(item.origin, Some(store.all()[0].id));
        assert_eq!(item.first_event_at, first.first_event_at);
        let sent = published_events(&mut published);
        assert_eq!(created_frames(sent.clone()), Vec::<String>::new());
        assert_eq!(sent.len(), 1, "one needs_you.changed for the second bell");
        assert!(sent[0].contains("needs_you.changed"), "{}", sent[0]);
    }

    #[test]
    fn the_item_follows_the_notification_event_with_its_origin() {
        let agent = agent_named("Ana");
        let (bridge, store, _agents, hub) = bridge_for(&agent, Some(RunId(1)));
        let mut published = hub.subscribe();

        bridge.apply_frame(&frame_of("waiting_for_input", &agent));

        let sent = published_events(&mut published);
        assert!(sent[0].contains("notification.created"), "{}", sent[0]);
        assert!(sent[1].contains("needs_you.changed"), "{}", sent[1]);
        let item = bridge.needs_you.items().remove(0);
        assert_eq!(item.origin, Some(store.all()[0].id));
    }

    #[test]
    fn the_detail_of_each_kind_is_the_one_the_contract_names() {
        let waiting = agent_named("Ana");
        let (bridge, ..) = bridge_for(&waiting, Some(RunId(1)));
        bridge.apply_frame(&frame_of("waiting_for_input", &waiting));
        assert_eq!(
            kinds_and_details(&bridge),
            [("waiting_for_input".to_owned(), Some("5000".to_owned()))]
        );

        let failed = agent_named("Bo");
        let (bridge, ..) = bridge_for(&failed, Some(RunId(1)));
        bridge.apply_frame(&frame_of_exit(&failed, 2));
        assert_eq!(
            kinds_and_details(&bridge),
            [(
                "failed".to_owned(),
                Some("the command ended with exit code 2".to_owned())
            )]
        );

        let exited = agent_named("Eli");
        let (bridge, ..) = bridge_for(&exited, Some(RunId(1)));
        bridge.apply_frame(&frame_of_exit(&exited, 0));
        assert_eq!(kinds_and_details(&bridge), [("exited".to_owned(), None)]);

        let held = agent_named("Cy");
        let (bridge, ..) = bridge_for(&held, None);
        bridge.record_auto_hold(held.id, "creates");
        assert_eq!(
            kinds_and_details(&bridge),
            [("auto_hold".to_owned(), Some("creates".to_owned()))]
        );

        let ringing = agent_named("Di");
        let (bridge, ..) = bridge_for(&ringing, Some(RunId(1)));
        bridge.apply_bell(bell_of(&ringing));
        assert_eq!(kinds_and_details(&bridge), [("bell".to_owned(), None)]);
    }

    #[test]
    fn stopped_and_interrupted_add_no_item() {
        for status in ["stopped", "interrupted"] {
            let agent = agent_named("Ana");
            let (bridge, ..) = bridge_for(&agent, Some(RunId(1)));

            bridge.apply_frame(&frame_of(status, &agent));

            assert_eq!(bridge.needs_you.items(), [], "{status}");
        }
    }

    #[test]
    fn interrupted_keeps_an_item_and_stopped_working_and_starting_clear_it() {
        let agent = agent_named("Ana");
        let (bridge, ..) = bridge_for(&agent, Some(RunId(1)));
        bridge.apply_frame(&frame_of("waiting_for_input", &agent));

        bridge.apply_frame(&frame_of("interrupted", &agent));
        assert_eq!(bridge.needs_you.items().len(), 1);

        for status in [
            AgentStatus::Starting,
            AgentStatus::Working,
            AgentStatus::Stopped,
        ] {
            bridge.apply_frame(&frame_of("waiting_for_input", &agent));
            // A person's input prompts the `working` that follows a wait.
            bridge.waits.wait_end(agent.id);
            bridge.apply_frame(&status_frame(agent.id, status));
            assert_eq!(bridge.needs_you.items(), [], "{status:?}");
        }
    }

    #[test]
    fn working_after_waiting_for_input_clears_the_item_and_tells_the_clients() {
        let agent = agent_named("Ana");
        let (bridge, _store, _agents, hub) = bridge_for(&agent, Some(RunId(1)));
        bridge.apply_frame(&frame_of("waiting_for_input", &agent));
        let mut published = hub.subscribe();
        bridge.waits.wait_end(agent.id);

        bridge.apply_frame(&status_frame(agent.id, AgentStatus::Working));

        assert_eq!(bridge.needs_you.items(), []);
        let sent = published_events(&mut published);
        assert_eq!(sent.len(), 1);
        assert!(sent[0].contains("needs_you.changed"), "{}", sent[0]);
        assert!(sent[0].contains("\"items\":[]"), "{}", sent[0]);
    }

    #[tokio::test]
    async fn an_unprompted_working_leaves_the_item_alone() {
        let agent = agent_named("Ana");
        let (bridge, ..) = bridge_for(&agent, Some(RunId(1)));
        bridge.apply_frame(&frame_of("waiting_for_input", &agent));
        let waiting = bridge.needs_you.items();

        bridge.apply_frame(&status_frame(agent.id, AgentStatus::Working));

        assert_eq!(bridge.needs_you.items(), waiting);
    }

    #[test]
    fn an_unprompted_wait_adds_no_notification_and_leaves_the_item_alone() {
        let agent = agent_named("Ana");
        let (bridge, store, ..) = bridge_for(&agent, Some(RunId(1)));
        bridge.apply_frame(&frame_of("waiting_for_input", &agent));
        bridge.apply_frame(&frame_of("failed", &agent));
        let failed = bridge.needs_you.items();
        let notifications = store.all().len();

        bridge.apply_frame(&frame_of("waiting_for_input", &agent));

        assert_eq!(bridge.needs_you.items(), failed);
        assert_eq!(store.all().len(), notifications);
    }

    #[test]
    fn a_working_that_follows_starting_is_not_judged_and_clears_the_item() {
        let agent = agent_named("Ana");
        let (bridge, ..) = bridge_for(&agent, Some(RunId(1)));
        bridge.apply_frame(&frame_of("waiting_for_input", &agent));

        bridge.apply_frame(&status_frame(agent.id, AgentStatus::Starting));
        bridge.apply_frame(&frame_of("waiting_for_input", &agent));
        bridge.apply_frame(&status_frame(agent.id, AgentStatus::Starting));
        bridge.apply_frame(&status_frame(agent.id, AgentStatus::Working));

        assert_eq!(bridge.needs_you.items(), []);
        assert_eq!(bridge.waits.working_since(agent.id), None);
    }

    #[test]
    fn a_read_right_after_notification_created_finds_the_item_and_clears_it() {
        let agent = agent_named("Ana");
        let (bridge, store, _agents, hub) = bridge_for(&agent, Some(RunId(1)));
        let mut published = hub.subscribe();
        let queue = bridge.needs_you.clone();
        let notifications = Arc::clone(&store);
        // The desktop of a focused terminal reads the notification the moment
        // it hears of it.
        let reading_client = std::thread::spawn(move || {
            loop {
                let frame = published.blocking_recv().expect("an event");
                if frame.contains("notification.created") {
                    queue.clear_on_read(notifications.all()[0].id);
                    return;
                }
            }
        });

        bridge.apply_frame(&frame_of("failed", &agent));
        reading_client.join().expect("the reading client");

        assert_eq!(bridge.needs_you.items(), []);
    }

    #[test]
    fn a_deleted_agent_keeps_its_notification_but_gets_no_item() {
        let agent = agent_named("Ana");
        let (bridge, store, agents, hub) = bridge_for(&agent, Some(RunId(1)));
        agents.deleted.store(true, Ordering::SeqCst);
        let mut published = hub.subscribe();

        bridge.apply_frame(&frame_of("failed", &agent));

        assert_eq!(bridge.needs_you.items(), []);
        assert_eq!(store.all().len(), 1);
        let sent = published_events(&mut published);
        assert_eq!(sent.len(), 1, "only notification.created: {sent:?}");
        assert!(sent[0].contains("notification.created"), "{}", sent[0]);
    }

    #[test]
    fn a_coalesced_bell_after_a_clear_makes_an_item_with_a_null_origin_on_the_wire() {
        let agent = agent_named("Ana");
        let (bridge, _store, _agents, hub) = bridge_for(&agent, Some(RunId(1)));
        bridge.apply_bell(bell_of(&agent));
        bridge
            .needs_you
            .clear(agent.id, NeedsYouClearReason::TerminalOpened);
        let mut published = hub.subscribe();

        bridge.apply_bell(bell_of(&agent));

        assert_eq!(bridge.needs_you.items().remove(0).origin, None);
        let sent = published_events(&mut published);
        assert!(
            sent[0].contains("\"origin_notification_id\":null"),
            "{}",
            sent[0]
        );
    }
}

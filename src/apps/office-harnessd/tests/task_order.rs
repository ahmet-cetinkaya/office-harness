// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! The order of the task texts (0011): a directive receipt waits while the
//! gateway holds the task order for a user edit, so clients see the
//! `agent.task` events in the order the texts were stored. The store is a
//! fake that blocks the user's write, so the test needs no timing luck to
//! hold the gateway inside its critical section.

mod support;

use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use agent_application::commands::set_task_text::SetTaskTextHandler;
use agent_application::common::errors::PortError;
use agent_application::common::interfaces::AgentStore;
use agent_domain::entities::Agent;
use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::{TaskSource, TaskText};
use agent_presentation::handlers::AgentHandlers;
use hierarchy_application::common::ports::DirectiveReceipt;
use kernel::AgentId;
use office_harnessd::bootstrap::{self, LoginSetup, Services};
use office_harnessd::bridges::{DirectiveReceiptBridge, SystemClock};
use office_harnessd::server::{AgentGateway, HoldGateway};
use serde_json::json;
use support::{agent_named, config_in};
use wire::{Request, Response};

/// How long the test waits for a step that must happen.
const STEP_WAIT: Duration = Duration::from_secs(10);
/// How long the test lets the receipt try before it checks that it still
/// waits. A receipt that skips the lock would store within this time.
const RECEIPT_TRY_WAIT: Duration = Duration::from_millis(300);

const USER_TEXT: &str = "Edited by the user";
const DIRECTIVE_TEXT: &str = "Fix the login test";

/// Stores through the real store, except that the first write of a
/// user's text stops until the test lets it go. Every write is logged in the
/// order it reached the store.
struct BlockingStore {
    inner: Arc<dyn AgentStore>,
    entered: Mutex<Sender<()>>,
    release: Mutex<Receiver<()>>,
    writes: Mutex<Vec<TaskSource>>,
}

impl AgentStore for BlockingStore {
    fn insert(&self, agent: &Agent) -> Result<(), PortError> {
        self.inner.insert(agent)
    }

    fn find(&self, id: AgentId) -> Result<Option<Agent>, PortError> {
        self.inner.find(id)
    }

    fn list(&self) -> Result<Vec<Agent>, PortError> {
        self.inner.list()
    }

    fn update_status(&self, id: AgentId, status: AgentStatus) -> Result<(), PortError> {
        self.inner.update_status(id, status)
    }

    fn update_task(&self, id: AgentId, task: Option<&TaskText>) -> Result<(), PortError> {
        let source = task.map(TaskText::source);
        if source == Some(TaskSource::User) {
            self.entered
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .send(())
                .expect("the test waits for the user's write");
            self.release
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .recv_timeout(STEP_WAIT)
                .expect("the test lets the user's write go");
        }
        self.inner.update_task(id, task)?;
        self.writes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .extend(source);
        Ok(())
    }

    fn delete(&self, id: AgentId) -> Result<(), PortError> {
        self.inner.delete(id)
    }
}

/// The gateway over a store that blocks, sharing its task order with the
/// receipt bridge built by the same services.
fn gateway_over(services: &Services, store: &Arc<BlockingStore>) -> AgentGateway {
    let mut use_cases = services.agent.handlers.use_cases().clone();
    use_cases.set_task_text = SetTaskTextHandler::new(
        Arc::clone(store) as Arc<dyn AgentStore>,
        Arc::new(SystemClock),
        services.agent.journal.clone(),
    );
    let events = services.events.clone();
    let alerts = services.storage_alerts.clone();
    let holds = HoldGateway::new(
        Arc::clone(&services.hierarchy.holds),
        Arc::clone(&services.agent_bridge_store),
        services.agent.office_actions.clone(),
        events.clone(),
        alerts.clone(),
    );
    AgentGateway::new(
        Arc::new(AgentHandlers::new(use_cases)),
        services.agent.audit.clone(),
        events,
        alerts,
        Arc::clone(&services.agent.status_order),
        Arc::clone(&services.agent.task_order),
        Arc::clone(&services.agent.stopping),
        services.terminal.events.clone(),
        Arc::clone(&services.terminal.runtime),
        Arc::clone(&services.hierarchy.tree),
        holds,
        services.needs_you_bridge.clone(),
    )
}

fn task_events(frames: &mut tokio::sync::broadcast::Receiver<Arc<str>>) -> Vec<String> {
    let mut texts = Vec::new();
    while let Ok(frame) = frames.try_recv() {
        let event: serde_json::Value = serde_json::from_str(&frame).expect("event");
        if event["type"] == "agent.task" {
            texts.push(event["task"]["text"].as_str().expect("text").to_owned());
        }
    }
    texts
}

#[tokio::test(flavor = "multi_thread")]
async fn a_receipt_waits_for_a_user_edit_that_holds_the_task_order() {
    let folder = tempfile::tempdir().expect("folder");
    let config = config_in(&folder);
    let log = bootstrap::open_log(&config).expect("log");
    let services = bootstrap::start_with(&config, &log, LoginSetup::detect()).expect("start");
    let agent = agent_named(services.machine.id, "Ada");
    services.agent_bridge_store.insert(&agent).expect("agent");

    let (entered_sender, entered) = channel();
    let (release, release_receiver) = channel();
    let store = Arc::new(BlockingStore {
        inner: Arc::clone(&services.agent_bridge_store),
        entered: Mutex::new(entered_sender),
        release: Mutex::new(release_receiver),
        writes: Mutex::new(Vec::new()),
    });
    let gateway = gateway_over(&services, &store);
    let receipt = DirectiveReceiptBridge::new(
        Arc::clone(&store) as Arc<dyn AgentStore>,
        services.events.clone(),
        services.storage_alerts.clone(),
        Arc::clone(&services.agent.task_order),
    );
    let mut frames = services.events.subscribe();

    // The user's edit stops inside the store, holding the task order.
    let request = Request {
        id: 1,
        kind: "agent.task.set".to_owned(),
        fields: json!({"agent_id": agent.id.to_string(), "text": USER_TEXT})
            .as_object()
            .expect("fields")
            .clone(),
    };
    let edit = tokio::spawn(async move { gateway.handle(request).await });
    entered
        .recv_timeout(STEP_WAIT)
        .expect("the edit reached the store");

    // A directive arrives meanwhile and must not store before the edit ends.
    let (started_sender, started) = channel();
    let received = thread::spawn(move || {
        started_sender
            .send(())
            .expect("the test waits for the start");
        receipt.received(agent.id, DIRECTIVE_TEXT);
    });
    started
        .recv_timeout(STEP_WAIT)
        .expect("the receipt started");
    thread::sleep(RECEIPT_TRY_WAIT);
    assert!(
        !received.is_finished(),
        "the receipt must wait while the edit holds the task order"
    );
    assert!(
        store
            .writes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_empty(),
        "no text may reach the store before the edit"
    );

    release.send(()).expect("release the edit");
    let response = edit.await.expect("the request task");
    received.join().expect("the receipt thread");

    assert!(
        matches!(response, Response::Success { .. }),
        "the edit must succeed: {response:?}"
    );
    assert_eq!(
        *store.writes.lock().unwrap_or_else(PoisonError::into_inner),
        [TaskSource::User, TaskSource::Directive],
        "the stores follow the order the requests took the task order"
    );
    assert_eq!(
        task_events(&mut frames),
        [USER_TEXT, DIRECTIVE_TEXT],
        "the events follow the stores"
    );
}

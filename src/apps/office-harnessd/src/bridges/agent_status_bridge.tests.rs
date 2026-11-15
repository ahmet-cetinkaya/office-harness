// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn terminal_events_map_to_status_events() {
    assert_eq!(
        status_event(TerminalEvent::FirstOutput),
        Some((StatusEvent::FirstOutput, None))
    );
    assert_eq!(
        status_event(TerminalEvent::QuietPeriod),
        Some((StatusEvent::QuietPeriodElapsed, None))
    );
    assert_eq!(
        status_event(TerminalEvent::OutputResumed),
        Some((StatusEvent::Output, None))
    );
    assert_eq!(
        status_event(TerminalEvent::Exited { exit_code: 0 }),
        Some((StatusEvent::ExitedWithCodeZero, None))
    );
}

#[test]
fn a_failed_exit_names_the_exit_code() {
    let (event, detail) = status_event(TerminalEvent::Exited { exit_code: 3 }).expect("event");

    assert_eq!(event, StatusEvent::ExitedWithFailure);
    assert!(detail.is_some_and(|text| text.contains('3')));
}

#[test]
fn a_failed_output_file_changes_no_status() {
    assert_eq!(status_event(TerminalEvent::OutputStorageFailed), None);
}

mod runs {
    use std::sync::Arc;
    use std::time::Duration;

    use agent_application::commands::change_agent_status::ChangeAgentStatusHandler;
    use agent_application::common::errors::PortError;
    use agent_application::common::interfaces::AgentStore;
    use agent_domain::entities::Agent;
    use agent_domain::enums::AgentStatus;
    use agent_domain::value_objects::{AgentCommand, DisplayName, MachineId};
    use kernel::{AgentId, Timestamp};
    use terminal_application::common::errors::TerminalRuntimeError;
    use terminal_application::common::interfaces::{OutputSubscriber, TerminalRuntime};
    use terminal_application::common::models::{
        Attachment, RunId, SpawnTerminalRequest, SubscriptionId,
    };
    use terminal_domain::value_objects::{OutputKey, TerminalSize};

    use super::*;

    struct OneAgent(Mutex<Agent>);

    impl AgentStore for OneAgent {
        fn insert(&self, _agent: &Agent) -> Result<(), PortError> {
            Ok(())
        }

        fn find(&self, _id: AgentId) -> Result<Option<Agent>, PortError> {
            Ok(Some(self.0.lock().expect("lock").clone()))
        }

        fn list(&self) -> Result<Vec<Agent>, PortError> {
            Ok(Vec::new())
        }

        fn update_status(&self, _id: AgentId, status: AgentStatus) -> Result<(), PortError> {
            self.0.lock().expect("lock").status = status;
            Ok(())
        }

        fn delete(&self, _id: AgentId) -> Result<(), PortError> {
            Ok(())
        }
    }

    /// A runtime that only knows the current run.
    struct RunOnly(RunId);

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

        fn write_input(
            &self,
            agent_id: AgentId,
            _bytes: &[u8],
        ) -> Result<(), TerminalRuntimeError> {
            Err(TerminalRuntimeError::NotRunning(agent_id))
        }

        fn paste(&self, agent_id: AgentId, _text: &str) -> Result<(), TerminalRuntimeError> {
            Err(TerminalRuntimeError::NotRunning(agent_id))
        }

        fn resize(
            &self,
            agent_id: AgentId,
            _size: TerminalSize,
        ) -> Result<bool, TerminalRuntimeError> {
            Err(TerminalRuntimeError::NotRunning(agent_id))
        }

        fn stop(&self, agent_id: AgentId, _grace: Duration) -> Result<(), TerminalRuntimeError> {
            Err(TerminalRuntimeError::NotRunning(agent_id))
        }

        fn is_running(&self, _agent_id: AgentId) -> bool {
            true
        }

        fn current_run(&self, _agent_id: AgentId) -> Option<RunId> {
            Some(self.0)
        }
    }

    fn bridge_for(agent: &Agent, current: RunId) -> (AgentStatusBridge, Arc<OneAgent>) {
        let store = Arc::new(OneAgent(Mutex::new(agent.clone())));
        let log_folder = tempfile::tempdir().expect("folder");
        let config = crate::config::Config {
            data_folder: log_folder.path().join("office"),
        };
        let log = crate::bootstrap::open_log(&config).expect("log");
        let bridge = AgentStatusBridge {
            change_status: ChangeAgentStatusHandler::new(store.clone()),
            runtime: Arc::new(RunOnly(current)),
            hub: EventHub::new(),
            order: StatusOrder::default(),
            alerts: StorageAlerts::new(log),
        };
        (bridge, store)
    }

    fn running_agent() -> Agent {
        let mut agent = Agent::new(
            AgentId::generate(),
            DisplayName::parse("name", "Ana").expect("name"),
            AgentCommand::parse("claude").expect("command"),
            Some(std::env::temp_dir()),
            None,
            MachineId::generate(),
            Timestamp::from_unix_millis(1),
        )
        .expect("agent");
        agent.status = AgentStatus::Working;
        agent
    }

    #[test]
    fn a_late_exit_of_an_earlier_run_does_not_end_the_current_run() {
        let agent = running_agent();
        let (bridge, store) = bridge_for(&agent, RunId(2));

        bridge.apply(AgentTerminalEvent {
            agent_id: agent.id,
            run: RunId(1),
            event: TerminalEvent::Exited { exit_code: 1 },
        });

        assert_eq!(store.0.lock().expect("lock").status, AgentStatus::Working);
    }

    #[test]
    fn an_exit_of_the_current_run_ends_it() {
        let agent = running_agent();
        let (bridge, store) = bridge_for(&agent, RunId(2));

        bridge.apply(AgentTerminalEvent {
            agent_id: agent.id,
            run: RunId(2),
            event: TerminalEvent::Exited { exit_code: 0 },
        });

        assert_eq!(store.0.lock().expect("lock").status, AgentStatus::Exited);
    }
}

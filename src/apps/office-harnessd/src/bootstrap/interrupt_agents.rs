// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::PoisonError;
use std::sync::atomic::Ordering;

use agent_presentation::agent_status_event;
use tokio::task::spawn_blocking;

use super::directive_delivery::withdraw_waiting_directives;
use super::{ServerLog, Services};

/// Ends the running agents before the server stops (FR-004): each child is
/// stopped, and the agent and its session become `interrupted`. Every client
/// that is still connected hears about it. A child that cannot be ended is
/// logged, and the others still end. Waiting directives are withdrawn first,
/// with the server stop as the reason (0012/FR-005).
pub async fn interrupt_running_agents(services: &Services, log: &ServerLog) {
    withdraw_waiting_directives(&services.hierarchy.send_directive, &services.events, log).await;
    let handler = services.agent.interrupt_agents.clone();
    let order = std::sync::Arc::clone(&services.agent.status_order);
    let stopping = std::sync::Arc::clone(&services.agent.stopping);
    let outcome = spawn_blocking(move || {
        // The status bridge takes the same lock, so no terminal event changes
        // a status while the agents end.
        let _turn = order.lock().unwrap_or_else(PoisonError::into_inner);
        // Set under the lock: a start queued behind it sees the flag, and
        // none can begin after the agents ended.
        stopping.store(true, Ordering::SeqCst);
        handler.handle()
    })
    .await;
    match outcome {
        Ok(Ok(outcome)) => {
            for agent in &outcome.interrupted {
                services
                    .events
                    .publish(&agent_status_event(agent.id, agent.status, None));
            }
            for failure in &outcome.failures {
                log.error(&format!("an agent could not be interrupted: {failure}"));
            }
            log.info(&format!(
                "agents interrupted at stop: {}",
                outcome.interrupted.len()
            ));
        }
        Ok(Err(error)) => log.error(&format!("the agents could not be interrupted: {error}")),
        Err(error) => log.error(&format!("the interrupt task failed: {error}")),
    }
}

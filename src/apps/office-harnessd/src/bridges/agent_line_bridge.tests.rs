// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use std::sync::Arc;
use terminal_application::common::interfaces::TerminalEventSink;
use terminal_application::common::models::RunId;

/// The lines the hub published since the receiver subscribed, as (agent id,
/// line) pairs.
fn published_lines(
    receiver: &mut tokio::sync::broadcast::Receiver<Arc<str>>,
) -> Vec<(String, String)> {
    std::iter::from_fn(|| receiver.try_recv().ok())
        .filter_map(|frame| {
            let event = serde_json::from_str::<serde_json::Value>(&frame).ok()?;
            Some((
                event["agent_id"].as_str()?.to_owned(),
                event["line"].as_str()?.to_owned(),
            ))
        })
        .collect()
}

#[test]
fn a_changed_line_is_forwarded_with_the_agent_and_the_text() {
    let lines = TerminalEvents::new();
    let hub = EventHub::new();
    let agent = AgentId::generate();
    // The receiver is created before the forward: a broadcast channel
    // never replays the frames sent before it subscribed.
    let mut published = hub.subscribe();
    lines.publish_line(agent, RunId(1), "ready".to_owned());
    let mut bridge = AgentLineBridge::new(lines.clone(), hub.clone());
    bridge.forward_new_lines();
    assert_eq!(
        published_lines(&mut published),
        vec![(agent.to_string(), "ready".to_owned())]
    );
}

#[test]
fn the_same_line_is_not_forwarded_twice_but_a_new_one_is() {
    let lines = TerminalEvents::new();
    let hub = EventHub::new();
    let agent = AgentId::generate();
    let mut published = hub.subscribe();
    lines.publish_line(agent, RunId(1), "ready".to_owned());
    let mut bridge = AgentLineBridge::new(lines.clone(), hub.clone());
    bridge.forward_new_lines();
    bridge.forward_new_lines();
    assert_eq!(published_lines(&mut published).len(), 1);
    lines.publish_line(agent, RunId(1), "done".to_owned());
    bridge.forward_new_lines();
    assert_eq!(
        published_lines(&mut published),
        vec![(agent.to_string(), "done".to_owned())]
    );
}

#[test]
fn a_new_run_overwrites_the_old_line() {
    let lines = TerminalEvents::new();
    let hub = EventHub::new();
    let agent = AgentId::generate();
    let mut published = hub.subscribe();
    lines.publish_line(agent, RunId(1), "old".to_owned());
    let mut bridge = AgentLineBridge::new(lines.clone(), hub.clone());
    bridge.forward_new_lines();
    lines.publish_line(agent, RunId(2), "new".to_owned());
    bridge.forward_new_lines();
    let sent = published_lines(&mut published);
    assert_eq!(sent.last(), Some(&(agent.to_string(), "new".to_owned())));
}

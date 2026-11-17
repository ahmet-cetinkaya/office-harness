// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex};

use super::*;
use hierarchy_domain::{Role, TreeRow};

use crate::common::interfaces::{TreeExclusion, TreeSnapshot};

fn locked<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The store reads are the only calls these tests make beyond the ports.
struct FixedRows {
    rows: Vec<TreeRow>,
    /// Never contended here; the exclusion exists so the trait stays whole.
    tree_writes: Mutex<()>,
}

impl HierarchyStore for FixedRows {
    fn snapshot(&self) -> Result<TreeSnapshot, HierarchyApplicationError> {
        Ok(TreeSnapshot {
            rows: self.rows.clone(),
            teams: Vec::new(),
        })
    }

    fn tree_exclusion(&self) -> TreeExclusion<'_> {
        TreeExclusion::new(&self.tree_writes)
    }

    fn team_name_taken(&self, _key: &str) -> Result<bool, HierarchyApplicationError> {
        panic!("not used in these tests")
    }

    fn save_team_and_place_lead(
        &self,
        _team: &hierarchy_domain::Team,
        _parent_agent_id: Option<AgentId>,
    ) -> Result<(), HierarchyApplicationError> {
        panic!("not used in these tests")
    }

    fn delete_team_and_unplace(
        &self,
        _team_id: hierarchy_domain::TeamId,
    ) -> Result<(), HierarchyApplicationError> {
        panic!("not used in these tests")
    }

    fn place_agent(
        &self,
        _agent_id: AgentId,
        _role: Option<Role>,
        _parent_agent_id: Option<AgentId>,
        _team_id: Option<hierarchy_domain::TeamId>,
    ) -> Result<(), HierarchyApplicationError> {
        panic!("not used in these tests")
    }

    fn team_name_of(
        &self,
        _team_id: hierarchy_domain::TeamId,
    ) -> Result<Option<hierarchy_domain::TeamName>, HierarchyApplicationError> {
        panic!("not used in these tests")
    }
}

/// One written line, as the recorder holds it.
#[derive(Clone)]
struct Written {
    agent_id: AgentId,
    text: String,
}

/// One journaled attempt, as the recorder holds it.
#[derive(Clone)]
struct Journaled {
    sender: AgentId,
    target: AgentId,
    text: String,
    failure: Option<String>,
}

/// Targets everything unless told one agent is down, and knows the family's
/// names the way the real bridge does.
struct ToggledTargets {
    stopped: Mutex<Option<AgentId>>,
    names: Vec<(AgentId, &'static str)>,
}

impl ToggledTargets {
    fn all_running() -> Self {
        Self::with_names(Vec::new())
    }

    fn with_stopped(agent_id: AgentId) -> Self {
        Self::with_stopped_and_names(agent_id, Vec::new())
    }

    fn with_names(names: Vec<(AgentId, &'static str)>) -> Self {
        Self {
            stopped: Mutex::new(None),
            names,
        }
    }

    fn with_stopped_and_names(agent_id: AgentId, names: Vec<(AgentId, &'static str)>) -> Self {
        Self {
            stopped: Mutex::new(Some(agent_id)),
            names,
        }
    }
}

impl DirectiveTarget for ToggledTargets {
    fn is_running(&self, agent_id: AgentId) -> bool {
        *locked(&self.stopped) != Some(agent_id)
    }

    fn name_of(&self, agent_id: AgentId) -> Option<String> {
        self.names
            .iter()
            .find(|(known, _)| *known == agent_id)
            .map(|(_, name)| (*name).to_owned())
    }
}

/// Records every write; fails once told which agent cannot take text.
struct RecordingWriter {
    written: Mutex<Vec<Written>>,
    failing: Mutex<Option<AgentId>>,
}

impl RecordingWriter {
    fn new(failing: Option<AgentId>) -> Self {
        Self {
            written: Mutex::new(Vec::new()),
            failing: Mutex::new(failing),
        }
    }

    fn lines(&self) -> Vec<Written> {
        locked(&self.written).clone()
    }
}

impl DirectiveWriter for RecordingWriter {
    fn write(&self, agent_id: AgentId, text: &str) -> Result<(), String> {
        if *locked(&self.failing) == Some(agent_id) {
            return Err("the session pipe is closed".to_owned());
        }
        locked(&self.written).push(Written {
            agent_id,
            text: text.to_owned(),
        });
        Ok(())
    }
}

/// Records every attempt in call order.
#[derive(Default)]
struct RecordingJournal {
    attempts: Mutex<Vec<Journaled>>,
}

impl RecordingJournal {
    fn attempts(&self) -> Vec<Journaled> {
        locked(&self.attempts).clone()
    }
}

impl DirectiveJournal for RecordingJournal {
    fn record(&self, sender: AgentId, target: AgentId, text: &str, failure: Option<&str>) {
        locked(&self.attempts).push(Journaled {
            sender,
            target,
            text: text.to_owned(),
            failure: failure.map(str::to_owned),
        });
    }
}

/// ceo > lead > member, the smallest tree where a member is a descendant of
/// a lead but not of another lead.
struct Family {
    lead: AgentId,
    other_lead: AgentId,
    member: AgentId,
    rows: Vec<TreeRow>,
}

fn family() -> Family {
    let ceo = AgentId::from_u128(1);
    let lead = AgentId::from_u128(2);
    let other_lead = AgentId::from_u128(3);
    let member = AgentId::from_u128(4);
    let row = |agent_id: AgentId, parent_agent_id: Option<AgentId>, role: Option<Role>| TreeRow {
        agent_id,
        role,
        parent_agent_id,
        team_id: None,
    };
    Family {
        lead,
        other_lead,
        member,
        rows: vec![
            row(ceo, None, Some(Role::Ceo)),
            row(lead, Some(ceo), Some(Role::Lead)),
            row(other_lead, Some(ceo), Some(Role::Lead)),
            row(member, Some(lead), Some(Role::Member)),
        ],
    }
}

struct Harness {
    handler: SendDirectiveHandler,
    writer: Arc<RecordingWriter>,
    journal: Arc<RecordingJournal>,
}

fn harness_for(family: &Family, targets: ToggledTargets, writer: RecordingWriter) -> Harness {
    let writer = Arc::new(writer);
    let journal = Arc::new(RecordingJournal::default());
    let handler = SendDirectiveHandler::new(
        Arc::new(FixedRows {
            rows: family.rows.clone(),
            tree_writes: Mutex::new(()),
        }),
        Arc::new(targets),
        writer.clone(),
        journal.clone(),
    );
    Harness {
        handler,
        writer,
        journal,
    }
}

#[test]
fn a_directive_to_a_running_descendant_is_written_and_journaled_clean() {
    let family = family();
    let harness = harness_for(
        &family,
        ToggledTargets::all_running(),
        RecordingWriter::new(None),
    );
    harness
        .handler
        .handle(family.lead, family.member, "please deploy")
        .expect("descendant write ok");
    let lines = harness.writer.lines();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].agent_id, family.member);
    assert_eq!(lines[0].text, "please deploy");
    let attempts = harness.journal.attempts();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].sender, family.lead);
    assert_eq!(attempts[0].target, family.member);
    assert_eq!(attempts[0].text, "please deploy");
    assert_eq!(attempts[0].failure, None);
}

#[test]
fn a_non_descendant_target_is_refused_before_any_write() {
    let family = family();
    let harness = harness_for(
        &family,
        ToggledTargets::all_running(),
        RecordingWriter::new(None),
    );
    let error = harness
        .handler
        .handle(family.lead, family.other_lead, "please deploy")
        .expect_err("sibling lead is not a descendant");
    assert!(matches!(
        error,
        HierarchyApplicationError::Invalid(HierarchyError::NotADescendant { .. })
    ));
    assert!(harness.writer.lines().is_empty());
    let attempts = harness.journal.attempts();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].target, family.other_lead);
    assert!(attempts[0].failure.is_some());
}

#[test]
fn the_sender_cannot_direct_itself() {
    let family = family();
    let harness = harness_for(
        &family,
        ToggledTargets::all_running(),
        RecordingWriter::new(None),
    );
    let error = harness
        .handler
        .handle(family.lead, family.lead, "please deploy")
        .expect_err("self is not a descendant");
    assert!(matches!(
        error,
        HierarchyApplicationError::Invalid(HierarchyError::NotADescendant { .. })
    ));
    assert!(harness.writer.lines().is_empty());
    assert_eq!(harness.journal.attempts().len(), 1);
}

#[test]
fn blank_text_is_refused_before_any_write() {
    let family = family();
    let harness = harness_for(
        &family,
        ToggledTargets::all_running(),
        RecordingWriter::new(None),
    );
    for text in ["", "   \t  "] {
        let error = harness
            .handler
            .handle(family.lead, family.member, text)
            .expect_err("blank text is refused");
        assert!(matches!(
            error,
            HierarchyApplicationError::Invalid(HierarchyError::Empty { .. })
        ));
    }
    assert!(harness.writer.lines().is_empty());
    assert_eq!(harness.journal.attempts().len(), 2);
}

#[test]
fn a_control_character_is_refused_before_any_write() {
    let family = family();
    let harness = harness_for(
        &family,
        ToggledTargets::all_running(),
        RecordingWriter::new(None),
    );
    let error = harness
        .handler
        .handle(family.lead, family.member, "deploy\u{7}now")
        .expect_err("control character is refused");
    assert!(matches!(
        error,
        HierarchyApplicationError::Invalid(HierarchyError::ControlCharacter { .. })
    ));
    assert!(harness.writer.lines().is_empty());
    // The audit keeps the rejected text as it was sent.
    let attempts = harness.journal.attempts();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].text, "deploy\u{7}now");
    assert!(attempts[0].failure.is_some());
}

#[test]
fn a_stopped_target_is_refused_by_name_before_any_write() {
    let family = family();
    let harness = harness_for(
        &family,
        ToggledTargets::with_stopped_and_names(
            family.member,
            vec![(family.member, "docker-watcher")],
        ),
        RecordingWriter::new(None),
    );
    let error = harness
        .handler
        .handle(family.lead, family.member, "please deploy")
        .expect_err("a stopped target cannot take text");
    assert_eq!(
        error,
        HierarchyApplicationError::Invalid(HierarchyError::NotRunning {
            name: "docker-watcher".to_owned()
        })
    );
    assert!(harness.writer.lines().is_empty());
    let attempts = harness.journal.attempts();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].text, "please deploy");
    assert!(attempts[0].failure.is_some());
}

#[test]
fn a_stopped_unknown_target_is_refused_by_id_before_any_write() {
    // The id only fails to resolve when the agent is gone from the office,
    // for example deleted between the snapshot and the lookup; the refusal
    // still names something.
    let family = family();
    let harness = harness_for(
        &family,
        ToggledTargets::with_stopped(family.member),
        RecordingWriter::new(None),
    );
    let error = harness
        .handler
        .handle(family.lead, family.member, "please deploy")
        .expect_err("a stopped target cannot take text");
    assert_eq!(
        error,
        HierarchyApplicationError::Invalid(HierarchyError::NotRunning {
            name: family.member.to_string()
        })
    );
    assert!(harness.writer.lines().is_empty());
    assert_eq!(harness.journal.attempts().len(), 1);
}

#[test]
fn a_writer_failure_surfaces_the_host_message_and_is_journaled() {
    let family = family();
    let harness = harness_for(
        &family,
        ToggledTargets::all_running(),
        RecordingWriter::new(Some(family.member)),
    );
    let error = harness
        .handler
        .handle(family.lead, family.member, "please deploy")
        .expect_err("the write fails");
    assert_eq!(
        error,
        HierarchyApplicationError::Conflict("the session pipe is closed".to_owned())
    );
    let attempts = harness.journal.attempts();
    assert_eq!(attempts.len(), 1);
    assert_eq!(
        attempts[0].failure.as_deref(),
        Some("conflict: the session pipe is closed")
    );
}

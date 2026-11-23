// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::atomic::{AtomicI64, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex};

use super::*;
use hierarchy_domain::{Role, TreeRow};
use kernel::{Actor, Timestamp};

use crate::commands::send_directive::{QUEUE_LIMIT, SETTLE_AFTER_TYPING, WAIT_LIMIT};

use crate::common::interfaces::{TreeExclusion, TreeSnapshot};
use crate::common::ports::DirectiveReceipt;

fn locked<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The store reads are the only calls these tests make beyond the ports.
struct FixedRows {
    rows: Mutex<Vec<TreeRow>>,
    /// Never contended here; the exclusion exists so the trait stays whole.
    tree_writes: Mutex<()>,
}

impl HierarchyStore for FixedRows {
    fn snapshot(&self) -> Result<TreeSnapshot, HierarchyApplicationError> {
        Ok(TreeSnapshot {
            rows: locked(&self.rows).clone(),
            teams: Vec::new(),
        })
    }

    fn tree_exclusion(&self) -> TreeExclusion<'_> {
        TreeExclusion::new(&self.tree_writes)
    }

    fn team_name_taken(&self, _key: &str) -> Result<bool, HierarchyApplicationError> {
        panic!("not used in these tests")
    }

    fn save_team_and_place_lead_held(
        &self,
        _exclusion: &TreeExclusion<'_>,
        _team: &hierarchy_domain::Team,
        _parent_agent_id: Option<AgentId>,
    ) -> Result<(), HierarchyApplicationError> {
        panic!("not used in these tests")
    }

    fn delete_team_and_unplace_held(
        &self,
        _exclusion: &TreeExclusion<'_>,
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
    readiness: Mutex<Readiness>,
    asleep: Mutex<Option<AgentId>>,
    temps: Vec<AgentId>,
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
            readiness: Mutex::new(Readiness::Ready),
            asleep: Mutex::new(None),
            temps: Vec::new(),
        }
    }

    fn with_temp(mut self, agent_id: AgentId) -> Self {
        self.temps.push(agent_id);
        self
    }

    fn with_stopped_and_names(agent_id: AgentId, names: Vec<(AgentId, &'static str)>) -> Self {
        Self {
            stopped: Mutex::new(Some(agent_id)),
            names,
            readiness: Mutex::new(Readiness::Ready),
            asleep: Mutex::new(None),
            temps: Vec::new(),
        }
    }
}

impl DirectiveTarget for ToggledTargets {
    fn is_running(&self, agent_id: AgentId) -> Result<bool, String> {
        Ok(*locked(&self.stopped) != Some(agent_id))
    }

    fn is_asleep(&self, agent_id: AgentId) -> bool {
        *locked(&self.asleep) == Some(agent_id)
    }

    fn is_temp(&self, agent_id: AgentId) -> bool {
        self.temps.contains(&agent_id)
    }

    fn name_of(&self, agent_id: AgentId) -> Option<String> {
        self.names
            .iter()
            .find(|(known, _)| *known == agent_id)
            .map(|(_, name)| (*name).to_owned())
    }

    fn readiness(&self, _agent_id: AgentId) -> Result<Readiness, String> {
        Ok(*locked(&self.readiness))
    }
}

/// Records every write; fails once told which agent cannot take text.
struct RecordingWriter {
    written: Mutex<Vec<Written>>,
    failing: Mutex<Option<AgentId>>,
    delivery: Mutex<Delivery>,
}

impl RecordingWriter {
    fn new(failing: Option<AgentId>) -> Self {
        Self {
            written: Mutex::new(Vec::new()),
            failing: Mutex::new(failing),
            delivery: Mutex::new(Delivery::Submitted),
        }
    }

    fn lines(&self) -> Vec<Written> {
        locked(&self.written).clone()
    }
}

impl DirectiveWriter for RecordingWriter {
    fn deliver(&self, agent_id: AgentId, text: &str) -> Result<Delivery, String> {
        if *locked(&self.failing) == Some(agent_id) {
            return Err("the session pipe is closed".to_owned());
        }
        locked(&self.written).push(Written {
            agent_id,
            text: text.to_owned(),
        });
        Ok(*locked(&self.delivery))
    }
}

/// A steady clock the test moves by hand.
struct ManualClock {
    millis: AtomicI64,
}

impl ManualClock {
    fn new() -> Self {
        Self {
            millis: AtomicI64::new(1_700_000_000_000),
        }
    }

    fn advance(&self, span: std::time::Duration) {
        let millis = i64::try_from(span.as_millis()).expect("small span");
        self.millis.fetch_add(millis, AtomicOrdering::SeqCst);
    }
}

impl DirectiveClock for ManualClock {
    fn steady_now(&self) -> Timestamp {
        Timestamp::from_unix_millis(self.millis.load(AtomicOrdering::SeqCst))
    }
}

/// Records every attempt in call order.
#[derive(Default)]
struct RecordingJournal {
    attempts: Mutex<Vec<Journaled>>,
    objectives: Mutex<Vec<(Actor, AgentId)>>,
}

impl RecordingJournal {
    fn attempts(&self) -> Vec<Journaled> {
        locked(&self.attempts).clone()
    }

    fn objectives(&self) -> Vec<(Actor, AgentId)> {
        locked(&self.objectives).clone()
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

    fn record_objective(&self, creator: Actor, temp: AgentId) {
        locked(&self.objectives).push((creator, temp));
    }
}

/// Keeps every receipt in call order, and every write of the writer that
/// shares its log, so a test can compare the two orders.
#[derive(Default)]
struct RecordingReceipt {
    received: Mutex<Vec<Written>>,
}

impl RecordingReceipt {
    fn received(&self) -> Vec<Written> {
        locked(&self.received).clone()
    }
}

impl DirectiveReceipt for RecordingReceipt {
    fn received(&self, target: AgentId, text: &str) {
        locked(&self.received).push(Written {
            agent_id: target,
            text: text.to_owned(),
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
        name: agent_id.to_string(),
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
    store: Arc<FixedRows>,
    targets: Arc<ToggledTargets>,
    writer: Arc<RecordingWriter>,
    journal: Arc<RecordingJournal>,
    receipt: Arc<RecordingReceipt>,
    clock: Arc<ManualClock>,
}

impl Harness {
    fn set_readiness(&self, readiness: Readiness) {
        *locked(&self.targets.readiness) = readiness;
    }

    /// Runs delivery rounds, a settle gap apart, until nothing waits.
    fn deliver_all(&self) {
        for _ in 0..=QUEUE_LIMIT {
            self.clock.advance(SETTLE_AFTER_TYPING);
            self.handler.deliver_waiting();
        }
    }

    fn texts_written(&self) -> Vec<String> {
        self.writer
            .lines()
            .into_iter()
            .map(|line| line.text)
            .collect()
    }

    fn failures(&self) -> Vec<String> {
        self.journal
            .attempts()
            .into_iter()
            .filter_map(|attempt| attempt.failure)
            .collect()
    }
}

fn harness_for(family: &Family, targets: ToggledTargets, writer: RecordingWriter) -> Harness {
    let store = Arc::new(FixedRows {
        rows: Mutex::new(family.rows.clone()),
        tree_writes: Mutex::new(()),
    });
    let targets = Arc::new(targets);
    let writer = Arc::new(writer);
    let journal = Arc::new(RecordingJournal::default());
    let receipt = Arc::new(RecordingReceipt::default());
    let clock = Arc::new(ManualClock::new());
    let handler = SendDirectiveHandler::new(DirectivePorts {
        store: store.clone(),
        targets: targets.clone(),
        writer: writer.clone(),
        journal: journal.clone(),
        receipt: receipt.clone(),
        clock: clock.clone(),
    });
    Harness {
        handler,
        store,
        targets,
        writer,
        journal,
        receipt,
        clock,
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
    let outcome = harness
        .handler
        .handle(family.lead, family.member, "please deploy")
        .expect("descendant write ok");
    assert_eq!(outcome, DirectiveOutcome::Submitted);
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
    let received = harness.receipt.received();
    assert_eq!(received.len(), 1);
    assert_eq!(received[0].agent_id, family.member);
    assert_eq!(received[0].text, "please deploy");
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
fn an_asleep_target_is_refused_as_asleep_and_journaled() {
    let family = family();
    let targets = ToggledTargets::with_stopped_and_names(
        family.member,
        vec![(family.member, "docker-watcher")],
    );
    *locked(&targets.asleep) = Some(family.member);
    let harness = harness_for(&family, targets, RecordingWriter::new(None));

    let error = harness
        .handler
        .handle(family.lead, family.member, "please deploy")
        .expect_err("an asleep target cannot take text");

    assert_eq!(
        error,
        HierarchyApplicationError::Invalid(HierarchyError::Asleep {
            name: "docker-watcher".to_owned()
        })
    );
    assert!(harness.writer.lines().is_empty());
    assert_eq!(
        harness.failures(),
        ["invalid: the agent is not running, it is asleep: docker-watcher"]
    );
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

#[test]
fn a_writer_failure_reports_no_receipt() {
    let family = family();
    let harness = harness_for(
        &family,
        ToggledTargets::all_running(),
        RecordingWriter::new(Some(family.member)),
    );

    harness
        .handler
        .handle(family.lead, family.member, "please deploy")
        .expect_err("the write fails");

    assert!(harness.receipt.received().is_empty());
}

#[test]
fn a_refused_directive_reports_no_receipt() {
    let family = family();
    let harness = harness_for(
        &family,
        ToggledTargets::with_stopped(family.member),
        RecordingWriter::new(None),
    );

    for (target, text) in [
        (family.other_lead, "please deploy"),
        (family.member, ""),
        (family.member, "deploy\u{7}now"),
        (family.member, "please deploy"),
    ] {
        harness
            .handler
            .handle(family.lead, target, text)
            .expect_err("refused");
    }

    assert!(harness.receipt.received().is_empty());
}

#[test]
fn directives_sent_at_once_are_received_in_the_order_they_were_written() {
    const SENDERS: usize = 8;
    let family = family();
    let harness = Arc::new(harness_for(
        &family,
        ToggledTargets::all_running(),
        RecordingWriter::new(None),
    ));

    std::thread::scope(|scope| {
        for number in 0..SENDERS {
            let harness = Arc::clone(&harness);
            let (lead, member) = (family.lead, family.member);
            scope.spawn(move || {
                harness
                    .handler
                    .handle(lead, member, &format!("task {number}"))
                    .expect("sent");
            });
        }
    });

    // The first takes the empty target; the others wait and follow.
    harness.deliver_all();
    let texts = |lines: Vec<Written>| lines.into_iter().map(|line| line.text).collect::<Vec<_>>();
    assert_eq!(texts(harness.receipt.received()).len(), SENDERS);
    assert_eq!(
        texts(harness.receipt.received()),
        texts(harness.writer.lines())
    );
}

fn ready_harness(family: &Family) -> Harness {
    harness_for(
        family,
        ToggledTargets::with_names(vec![(family.member, "Member")]),
        RecordingWriter::new(None),
    )
}

fn send(harness: &Harness, family: &Family, text: &str) -> DirectiveOutcome {
    harness
        .handler
        .handle(family.lead, family.member, text)
        .expect("accepted")
}

#[test]
fn a_busy_target_queues_the_directive_and_types_nothing() {
    let family = family();
    let harness = ready_harness(&family);
    harness.set_readiness(Readiness::Busy);

    assert_eq!(
        send(&harness, &family, "please deploy"),
        DirectiveOutcome::Waiting { position: 1 }
    );
    assert_eq!(
        send(&harness, &family, "then test"),
        DirectiveOutcome::Waiting { position: 2 }
    );

    assert!(harness.writer.lines().is_empty());
    assert!(harness.receipt.received().is_empty());
    // The acceptance is the one entry of each directive (PB-2).
    let attempts = harness.journal.attempts();
    assert_eq!(attempts.len(), 2);
    assert!(attempts.iter().all(|attempt| attempt.failure.is_none()));
}

#[test]
fn a_waiting_directive_is_typed_once_the_target_is_ready() {
    let family = family();
    let harness = ready_harness(&family);
    harness.set_readiness(Readiness::Busy);
    send(&harness, &family, "please deploy");
    harness.handler.deliver_waiting();
    assert!(harness.writer.lines().is_empty());

    harness.set_readiness(Readiness::Ready);
    harness.handler.deliver_waiting();

    assert_eq!(harness.texts_written(), ["please deploy"]);
    assert_eq!(harness.receipt.received().len(), 1);
    assert_eq!(harness.journal.attempts().len(), 1);
}

#[test]
fn ten_waiting_directives_are_typed_once_each_in_the_order_sent() {
    let family = family();
    let harness = ready_harness(&family);
    harness.set_readiness(Readiness::Busy);
    let sent: Vec<String> = (0..10).map(|number| format!("task {number}")).collect();
    for text in &sent {
        send(&harness, &family, text);
    }

    harness.set_readiness(Readiness::Ready);
    harness.deliver_all();

    assert_eq!(harness.texts_written(), sent);
}

#[test]
fn the_next_directive_waits_the_settle_gap_after_one_was_typed() {
    let family = family();
    let harness = ready_harness(&family);
    assert_eq!(
        send(&harness, &family, "first"),
        DirectiveOutcome::Submitted
    );
    assert_eq!(
        send(&harness, &family, "second"),
        DirectiveOutcome::Waiting { position: 1 }
    );
    harness.handler.deliver_waiting();
    assert_eq!(harness.texts_written(), ["first"]);

    harness.clock.advance(SETTLE_AFTER_TYPING);
    harness.handler.deliver_waiting();

    assert_eq!(harness.texts_written(), ["first", "second"]);
}

#[test]
fn a_full_queue_refuses_the_next_directive_and_records_it() {
    let family = family();
    let harness = ready_harness(&family);
    harness.set_readiness(Readiness::Busy);
    for number in 0..QUEUE_LIMIT {
        send(&harness, &family, &format!("task {number}"));
    }

    let error = harness
        .handler
        .handle(family.lead, family.member, "one more")
        .expect_err("the queue is full");

    let reason = format!("the queue of Member is full: {QUEUE_LIMIT} directives wait");
    assert_eq!(error, HierarchyApplicationError::Refused(reason.clone()));
    assert_eq!(harness.failures(), [reason]);
}

#[test]
fn a_directive_that_waits_ten_minutes_is_withdrawn_untyped() {
    let family = family();
    let harness = ready_harness(&family);
    harness.set_readiness(Readiness::Busy);
    send(&harness, &family, "please deploy");

    harness.clock.advance(WAIT_LIMIT);
    harness.handler.deliver_waiting();
    harness.set_readiness(Readiness::Ready);
    harness.deliver_all();

    assert!(harness.writer.lines().is_empty());
    assert_eq!(
        harness.failures(),
        ["withdrawn before typing: not typed within 10 minutes"]
    );
    let withdrawal = harness.journal.attempts().pop().expect("withdrawal");
    assert_eq!(withdrawal.sender, family.lead);
    assert_eq!(withdrawal.target, family.member);
    assert_eq!(withdrawal.text, "please deploy");
}

#[test]
fn a_target_that_stopped_withdraws_every_waiting_directive() {
    let family = family();
    let harness = ready_harness(&family);
    harness.set_readiness(Readiness::Busy);
    send(&harness, &family, "first");
    send(&harness, &family, "second");

    harness.set_readiness(Readiness::Gone);
    harness.handler.deliver_waiting();

    assert!(harness.writer.lines().is_empty());
    assert_eq!(
        harness.failures(),
        [
            "withdrawn before typing: Member is no longer running",
            "withdrawn before typing: Member is no longer running",
        ]
    );
}

#[test]
fn a_target_that_went_to_sleep_withdraws_every_waiting_directive_as_asleep() {
    let family = family();
    let harness = ready_harness(&family);
    harness.set_readiness(Readiness::Busy);
    send(&harness, &family, "first");
    send(&harness, &family, "second");
    assert_eq!(harness.handler.waiting_count(family.member), 2);

    harness.set_readiness(Readiness::Gone);
    *locked(&harness.targets.asleep) = Some(family.member);
    harness.handler.deliver_waiting();

    assert!(harness.writer.lines().is_empty());
    assert_eq!(harness.handler.waiting_count(family.member), 0);
    assert_eq!(
        harness.failures(),
        [
            "withdrawn before typing: the agent is not running, it is asleep: Member",
            "withdrawn before typing: the agent is not running, it is asleep: Member",
        ]
    );
}

#[test]
fn a_target_moved_out_of_the_subtree_withdraws_the_waiting_directive() {
    let family = family();
    let harness = ready_harness(&family);
    harness.set_readiness(Readiness::Busy);
    send(&harness, &family, "please deploy");
    for row in locked(&harness.store.rows).iter_mut() {
        if row.agent_id == family.member {
            row.parent_agent_id = Some(family.other_lead);
        }
    }

    harness.set_readiness(Readiness::Ready);
    harness.handler.deliver_waiting();

    assert!(harness.writer.lines().is_empty());
    let failures = harness.failures();
    assert_eq!(failures.len(), 1);
    assert!(
        failures[0].starts_with("withdrawn before typing: "),
        "{failures:?}"
    );
}

#[test]
fn a_directive_whose_text_does_not_show_is_left_unsubmitted() {
    let family = family();
    let harness = ready_harness(&family);
    *locked(&harness.writer.delivery) = Delivery::Unsubmitted;

    assert_eq!(
        send(&harness, &family, "please deploy"),
        DirectiveOutcome::Unsubmitted
    );

    assert!(harness.receipt.received().is_empty());
    let attempts = harness.journal.attempts();
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0].failure, None);
    assert_eq!(attempts[1].failure.as_deref(), Some(UNSUBMITTED));
}

#[test]
fn a_waiting_directive_whose_text_does_not_show_is_recorded_unsubmitted() {
    let family = family();
    let harness = ready_harness(&family);
    harness.set_readiness(Readiness::Busy);
    send(&harness, &family, "please deploy");
    *locked(&harness.writer.delivery) = Delivery::Unsubmitted;

    harness.set_readiness(Readiness::Ready);
    harness.handler.deliver_waiting();

    assert!(harness.receipt.received().is_empty());
    assert_eq!(harness.failures(), [UNSUBMITTED]);
}

#[test]
fn a_server_stop_withdraws_every_waiting_directive() {
    let family = family();
    let harness = ready_harness(&family);
    harness.set_readiness(Readiness::Busy);
    send(&harness, &family, "first");
    send(&harness, &family, "second");

    harness.handler.withdraw_all("the office server stopped");

    assert_eq!(
        harness.failures(),
        [
            "withdrawn before typing: the office server stopped",
            "withdrawn before typing: the office server stopped",
        ]
    );
    assert!(!harness.handler.wait_for_waiting(std::time::Duration::ZERO));
}

#[test]
fn a_directive_sent_after_a_server_stop_is_refused_and_recorded() {
    let family = family();
    let harness = ready_harness(&family);
    harness.handler.withdraw_all("the office server stopped");

    let error = harness
        .handler
        .handle(family.lead, family.member, "late task")
        .expect_err("the server is stopping");

    assert_eq!(
        error,
        HierarchyApplicationError::Refused(STOPPING.to_owned())
    );
    assert_eq!(harness.failures(), [STOPPING]);
    assert!(harness.writer.lines().is_empty());
    assert!(!harness.handler.wait_for_waiting(std::time::Duration::ZERO));
}

#[test]
fn a_head_claimed_before_a_server_stop_is_not_typed() {
    let family = family();
    let harness = ready_harness(&family);
    harness.set_readiness(Readiness::Busy);
    send(&harness, &family, "please deploy");
    // The delivery thread claims the head, and the stop withdraws it before
    // the thread types it.
    let head = harness
        .handler
        .queue
        .claim_head(family.member, harness.clock.steady_now())
        .head
        .expect("the head is claimed");
    harness.handler.withdraw_all("the office server stopped");

    harness.handler.deliver_head(&head);

    assert!(harness.writer.lines().is_empty());
    assert!(harness.receipt.received().is_empty());
    assert_eq!(
        harness.failures(),
        ["withdrawn before typing: the office server stopped"]
    );
}

const TEMP_REASON: &str = "idle for 30 minutes";

fn temp_harness(family: &Family, is_temp: bool) -> Harness {
    let mut targets = ToggledTargets::with_names(vec![(family.lead, "Lead")]);
    if is_temp {
        targets = targets.with_temp(family.member);
    }
    let harness = harness_for(family, targets, RecordingWriter::new(None));
    harness.set_readiness(Readiness::Busy);
    harness
}

fn parent_report(family: &Family) -> ParentReport {
    ParentReport {
        sender: family.member,
        parent: family.lead,
        line: "[report from Member: done] built".to_owned(),
        record_text: "done: built".to_owned(),
    }
}

fn remove_member(harness: &Harness, family: &Family) {
    locked(&harness.store.rows).retain(|row| row.agent_id != family.member);
}

/// What waits for `target`, oldest first, left in the queue.
fn waiting_of(harness: &Harness, target: AgentId) -> Vec<WaitingDirective> {
    let waiting = harness.handler.queue.drain(target);
    for entry in &waiting {
        harness.handler.queue.push(entry.clone());
    }
    waiting
}

#[test]
fn an_objective_waits_for_its_own_temp_and_is_recorded_once_with_the_creator() {
    let family = family();
    let harness = temp_harness(&family, true);
    let temp = family.member;

    let position = harness
        .handler
        .queue_objective(temp, "build the parser", Actor::User)
        .expect("queued");

    assert_eq!(position, 1);
    assert_eq!(harness.journal.objectives(), [(Actor::User, temp)]);
    assert!(harness.journal.attempts().is_empty());
    assert_eq!(
        waiting_of(&harness, temp)[0].origin,
        Origin::Objective { by: Actor::User }
    );
    harness.set_readiness(Readiness::Ready);
    harness.deliver_all();
    assert_eq!(harness.texts_written(), ["build the parser"]);
    assert_eq!(harness.receipt.received().len(), 1);
    assert_eq!(harness.journal.objectives().len(), 1);
}

#[test]
fn an_objective_is_refused_when_its_text_breaks_the_directive_rules() {
    let family = family();
    let harness = temp_harness(&family, true);

    let error = harness
        .handler
        .queue_objective(family.member, "", Actor::User)
        .expect_err("blank text");

    assert!(matches!(error, HierarchyApplicationError::Invalid(_)));
    assert_eq!(harness.journal.objectives(), Vec::<(Actor, AgentId)>::new());
    assert_eq!(harness.handler.waiting_count(family.member), 0);
}

#[test]
fn a_waiting_report_of_a_removed_temp_stays_and_is_typed_as_a_report() {
    let family = family();
    let harness = temp_harness(&family, true);
    harness
        .handler
        .send_report(&parent_report(&family), &family.rows)
        .expect("queued");
    let waiting = waiting_of(&harness, family.lead);
    assert_eq!(
        waiting[0].origin,
        Origin::Report {
            text: "done: built".to_owned()
        }
    );
    assert!(waiting[0].from_temp);

    remove_member(&harness, &family);
    harness.set_readiness(Readiness::Ready);
    harness.deliver_all();

    assert_eq!(
        harness.texts_written(),
        ["[report from Member: done] built"]
    );
    assert_eq!(harness.failures(), Vec::<String>::new());
}

#[test]
fn a_waiting_report_of_a_removed_non_temp_sender_is_withdrawn() {
    let family = family();
    let harness = temp_harness(&family, false);
    harness
        .handler
        .send_report(&parent_report(&family), &family.rows)
        .expect("queued");
    assert!(!waiting_of(&harness, family.lead)[0].from_temp);

    remove_member(&harness, &family);
    harness.set_readiness(Readiness::Ready);
    harness.deliver_all();

    assert!(harness.writer.lines().is_empty());
    assert_eq!(
        harness.failures(),
        ["withdrawn before typing: the sender is no longer a child of Lead"]
    );
}

#[test]
fn an_end_notice_of_a_removed_temp_stays_and_is_typed_as_an_end_notice() {
    let family = family();
    let harness = temp_harness(&family, true);
    let line = hierarchy_domain::end_notice_line("Member", family.member, TEMP_REASON);

    assert!(
        harness
            .handler
            .push_end_notice(family.member, family.lead, &line)
    );
    let waiting = waiting_of(&harness, family.lead);
    assert_eq!(waiting[0].origin, Origin::EndNotice { text: line.clone() });
    assert!(waiting[0].from_temp);

    remove_member(&harness, &family);
    harness.set_readiness(Readiness::Ready);
    harness.deliver_all();

    assert_eq!(harness.texts_written(), [line]);
    assert_eq!(harness.failures(), Vec::<String>::new());
    assert!(harness.receipt.received().is_empty());
}

#[test]
fn an_end_notice_takes_the_one_report_place_of_its_sender() {
    let family = family();
    let harness = temp_harness(&family, true);
    harness
        .handler
        .send_report(&parent_report(&family), &family.rows)
        .expect("queued");

    assert!(
        harness
            .handler
            .push_end_notice(family.member, family.lead, "[temp ended]")
    );

    assert_eq!(harness.handler.waiting_count(family.lead), 1);
    assert_eq!(
        harness.failures(),
        ["withdrawn before typing: replaced by a newer report"]
    );
}

#[test]
fn an_end_notice_is_not_queued_when_the_report_share_is_used_or_the_server_stops() {
    let family = family();
    let harness = temp_harness(&family, true);
    let other_temps = (100..).take(REPORT_SHARE).map(AgentId::from_u128);
    for temp in other_temps {
        assert!(
            harness
                .handler
                .push_end_notice(temp, family.lead, "[ended]")
        );
    }

    assert!(
        !harness
            .handler
            .push_end_notice(family.member, family.lead, "[ended]")
    );

    harness
        .handler
        .withdraw_all("the office server is stopping");
    assert!(
        !harness
            .handler
            .push_end_notice(family.member, family.lead, "[ended]")
    );
}

mod hold_guard {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use hierarchy_domain::{AUTO_HOLD_REFUSAL_COUNT, Hold, HoldScope, LimitRefusal, RateLimits};
    use kernel::Timestamp;

    use super::*;
    use crate::common::interfaces::HoldStore;
    use crate::common::ports::{HoldClock, HoldJournal, HoldNotifier, RateLimitSource};

    const NOW_MILLIS: i64 = 1_700_000_000_000;

    #[derive(Default)]
    struct NoStore;

    impl HoldStore for NoStore {
        fn load_holds(&self) -> Result<Vec<Hold>, HierarchyApplicationError> {
            Ok(Vec::new())
        }

        fn save_hold(&self, _hold: &Hold) -> Result<(), HierarchyApplicationError> {
            Ok(())
        }

        fn delete_hold(&self, _scope: HoldScope) -> Result<(), HierarchyApplicationError> {
            Ok(())
        }

        fn delete_agent_holds(
            &self,
            _agent_ids: &[AgentId],
        ) -> Result<(), HierarchyApplicationError> {
            Ok(())
        }
    }

    struct FixedLimits(RateLimits);

    impl RateLimitSource for FixedLimits {
        fn limits(&self) -> RateLimits {
            self.0
        }
    }

    struct FixedClock;

    impl HoldClock for FixedClock {
        fn now(&self) -> Timestamp {
            Timestamp::from_unix_millis(NOW_MILLIS)
        }

        fn steady_now(&self) -> Timestamp {
            Timestamp::from_unix_millis(NOW_MILLIS)
        }
    }

    /// Counts what the guard reports; the guard journals no refusal itself.
    #[derive(Default)]
    struct Counting {
        storage_failures: AtomicUsize,
        notifications: AtomicUsize,
    }

    impl HoldJournal for Counting {
        fn held(&self, _hold: &Hold) {}
        fn released(&self, _scope: HoldScope) {}
        fn storage_failed(&self, _message: &str) {
            self.storage_failures.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl HoldNotifier for Counting {
        fn auto_held(&self, _agent: AgentId, _limit: &LimitRefusal) {
            self.notifications.fetch_add(1, Ordering::SeqCst);
        }

        fn resume_held(&self, _agent: AgentId, _reason: &str) {}
    }

    struct Guarded {
        harness: Harness,
        service: Arc<HoldService>,
        counting: Arc<Counting>,
    }

    fn guarded(family: &Family, targets: ToggledTargets) -> Guarded {
        guarded_with(family, targets, RateLimits::default())
    }

    fn guarded_with(family: &Family, targets: ToggledTargets, limits: RateLimits) -> Guarded {
        let counting = Arc::new(Counting::default());
        let service = Arc::new(
            HoldService::new(
                Arc::new(NoStore),
                Arc::new(FixedLimits(limits)),
                counting.clone(),
                counting.clone(),
                Arc::new(FixedClock),
            )
            .expect("service"),
        );
        let mut harness = harness_for(family, targets, RecordingWriter::new(None));
        harness.handler = harness.handler.with_hold_guard(Arc::clone(&service));
        Guarded {
            harness,
            service,
            counting,
        }
    }

    #[test]
    fn a_held_office_refuses_a_directive_with_the_hold_reason() {
        let family = family();
        let guarded = guarded(&family, ToggledTargets::all_running());
        guarded.service.set_hold(HoldScope::Office).expect("hold");
        let error = guarded
            .harness
            .handler
            .handle(family.lead, family.member, "please deploy")
            .expect_err("the office is on hold");
        assert_eq!(
            error,
            HierarchyApplicationError::Refused("the office is on hold".to_owned())
        );
        assert!(guarded.harness.writer.lines().is_empty());
        let attempts = guarded.harness.journal.attempts();
        assert_eq!(attempts.len(), 1);
        assert_eq!(attempts[0].sender, family.lead);
        assert_eq!(attempts[0].target, family.member);
        assert_eq!(attempts[0].text, "please deploy");
        assert_eq!(
            attempts[0].failure.as_deref(),
            Some("the office is on hold")
        );
        assert_eq!(guarded.counting.storage_failures.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn a_non_descendant_target_gets_the_tree_refusal_not_the_hold_reason() {
        let family = family();
        let guarded = guarded(&family, ToggledTargets::all_running());
        guarded.service.set_hold(HoldScope::Office).expect("hold");
        let error = guarded
            .harness
            .handler
            .handle(family.lead, family.other_lead, "please deploy")
            .expect_err("sibling lead is not a descendant");
        assert!(matches!(
            error,
            HierarchyApplicationError::Invalid(HierarchyError::NotADescendant { .. })
        ));
        // The tree refusal is the one journaled attempt; the guard added none.
        assert_eq!(guarded.harness.journal.attempts().len(), 1);
    }

    #[test]
    fn directives_to_a_stopped_target_count_nothing_and_never_auto_hold() {
        let family = family();
        let guarded = guarded(&family, ToggledTargets::with_stopped(family.member));
        for _ in 0..RateLimits::default().directive {
            let _ = guarded
                .harness
                .handler
                .handle(family.lead, family.member, "please deploy");
        }
        assert!(guarded.harness.journal.attempts().iter().all(|attempt| {
            attempt
                .failure
                .as_deref()
                .is_some_and(|text| text.contains("not running"))
        }));
        assert_eq!(guarded.counting.notifications.load(Ordering::SeqCst), 0);
        assert_eq!(guarded.service.list().agents.len(), 0);
    }

    #[test]
    fn the_target_limit_refuses_the_fourth_directive_and_the_third_refusal_holds() {
        let family = family();
        let guarded = guarded(&family, ToggledTargets::all_running());
        let send = || {
            guarded
                .harness
                .handler
                .handle(family.lead, family.member, "please deploy")
        };
        for _ in 0..RateLimits::default().directive_target {
            send().expect("within the target limit");
        }
        for _ in 0..AUTO_HOLD_REFUSAL_COUNT {
            assert!(matches!(send(), Err(HierarchyApplicationError::Refused(_))));
        }
        assert_eq!(guarded.counting.notifications.load(Ordering::SeqCst), 1);
        assert_eq!(guarded.service.list().agents.len(), 1);
        let refusals: Vec<_> = guarded
            .harness
            .journal
            .attempts()
            .into_iter()
            .filter(|attempt| attempt.failure.is_some())
            .collect();
        assert_eq!(refusals.len(), AUTO_HOLD_REFUSAL_COUNT);
        assert!(
            refusals
                .iter()
                .all(|attempt| attempt.text == "please deploy")
        );
    }

    #[test]
    fn a_hold_set_while_a_directive_waits_withdraws_it_and_counts_nothing() {
        let family = family();
        let guarded = guarded(&family, ToggledTargets::all_running());
        guarded.harness.set_readiness(Readiness::Busy);
        guarded
            .harness
            .handler
            .handle(family.lead, family.member, "please deploy")
            .expect("queued");
        guarded.service.set_hold(HoldScope::Office).expect("hold");

        guarded.harness.set_readiness(Readiness::Ready);
        guarded.harness.handler.deliver_waiting();

        assert!(guarded.harness.writer.lines().is_empty());
        assert_eq!(
            guarded.harness.failures(),
            ["withdrawn before typing: the office is on hold"]
        );
        assert_eq!(guarded.counting.notifications.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn a_full_queue_refusal_never_counts_toward_a_rate_limit() {
        let family = family();
        let one_slot_over_the_queue = u32::try_from(QUEUE_LIMIT).expect("small") + 1;
        let limits = RateLimits {
            directive: one_slot_over_the_queue,
            directive_target: one_slot_over_the_queue,
            ..RateLimits::default()
        };
        let guarded = guarded_with(&family, ToggledTargets::all_running(), limits);
        guarded.harness.set_readiness(Readiness::Busy);
        let send = |text: &str| {
            guarded
                .harness
                .handler
                .handle(family.lead, family.member, text)
        };
        for number in 0..QUEUE_LIMIT {
            send(&format!("task {number}")).expect("within the queue and the limit");
        }
        for _ in 0..AUTO_HOLD_REFUSAL_COUNT {
            let refusal = send("one more").expect_err("the queue is full");
            assert!(refusal.to_string().contains("is full"), "{refusal}");
        }

        assert_eq!(guarded.counting.notifications.load(Ordering::SeqCst), 0);
        assert_eq!(guarded.service.list().agents.len(), 0);
        // The last rate-window slot is still free: the refusals took none.
        guarded.harness.set_readiness(Readiness::Ready);
        guarded.harness.deliver_all();
        send("the last slot").expect("one slot is left");
    }

    #[test]
    fn a_hold_set_while_the_persons_objective_waits_does_not_withdraw_it() {
        let family = family();
        let guarded = guarded(&family, ToggledTargets::all_running());
        guarded.harness.set_readiness(Readiness::Busy);
        guarded
            .harness
            .handler
            .queue_objective(family.member, "build it", Actor::User)
            .expect("queued");
        guarded.service.set_hold(HoldScope::Office).expect("hold");

        guarded.harness.set_readiness(Readiness::Ready);
        guarded.harness.handler.deliver_waiting();

        assert_eq!(guarded.harness.writer.lines().len(), 1);
        assert_eq!(guarded.harness.failures(), Vec::<String>::new());
    }

    #[test]
    fn a_hold_set_while_an_agents_objective_waits_withdraws_it() {
        let family = family();
        let guarded = guarded(&family, ToggledTargets::all_running());
        guarded.harness.set_readiness(Readiness::Busy);
        guarded
            .harness
            .handler
            .queue_objective(family.member, "build it", Actor::Agent(family.lead))
            .expect("queued");
        guarded.service.set_hold(HoldScope::Office).expect("hold");

        guarded.harness.set_readiness(Readiness::Ready);
        guarded.harness.handler.deliver_waiting();

        assert!(guarded.harness.writer.lines().is_empty());
        assert_eq!(
            guarded.harness.failures(),
            ["withdrawn before typing: the office is on hold"]
        );
    }

    #[test]
    fn objectives_count_toward_no_directive_limit_and_pass_a_hold() {
        let family = family();
        let limits = RateLimits {
            directive: 1,
            directive_target: 1,
            ..RateLimits::default()
        };
        let guarded = guarded_with(&family, ToggledTargets::all_running(), limits);
        guarded.harness.set_readiness(Readiness::Busy);
        guarded.service.set_hold(HoldScope::Office).expect("hold");

        for _ in 0..=AUTO_HOLD_REFUSAL_COUNT {
            guarded
                .harness
                .handler
                .queue_objective(family.member, "build it", Actor::User)
                .expect("an objective meets no limit and no hold");
        }

        assert_eq!(guarded.counting.notifications.load(Ordering::SeqCst), 0);
        assert_eq!(guarded.service.list().agents.len(), 0);
        assert!(guarded.harness.journal.attempts().is_empty());
    }
}

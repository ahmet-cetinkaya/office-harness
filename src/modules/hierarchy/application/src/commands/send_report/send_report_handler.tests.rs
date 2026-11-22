// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Mutex, MutexGuard, PoisonError};

use hierarchy_domain::{Hold, HoldScope, LimitRefusal, RateLimits, Role, TreeRow};
use kernel::Timestamp;

use super::*;
use crate::commands::send_directive::{DirectivePorts, REPORT_SHARE};
use crate::common::interfaces::{HoldStore, TreeExclusion, TreeSnapshot};
use crate::common::ports::{
    Delivery, DirectiveClock, DirectiveJournal, DirectiveReceipt, DirectiveWriter, HoldClock,
    HoldJournal, HoldNotifier, RateLimitSource, Readiness,
};

const CEO: u128 = 1;
const LEAD: u128 = 2;
const MEMBER: u128 = 3;
const OTHER_LEAD: u128 = 4;
/// The first of the extra members the queue-share test places under LEAD.
const FIRST_EXTRA_MEMBER: u128 = 100;
const NOW_MILLIS: i64 = 1_700_000_000_000;

fn id(value: u128) -> AgentId {
    AgentId::from_u128(value)
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn row(agent: u128, role: Role, parent: Option<u128>) -> TreeRow {
    TreeRow {
        agent_id: id(agent),
        name: agent.to_string(),
        role: Some(role),
        parent_agent_id: parent.map(id),
        team_id: None,
    }
}

fn company() -> Vec<TreeRow> {
    vec![
        row(CEO, Role::Ceo, None),
        row(LEAD, Role::Lead, Some(CEO)),
        row(OTHER_LEAD, Role::Lead, Some(CEO)),
        row(MEMBER, Role::Member, Some(LEAD)),
    ]
}

/// The tree as a test sets it; a test may move an agent between reads.
struct SettableRows {
    rows: Mutex<Vec<TreeRow>>,
    tree_writes: Mutex<()>,
}

impl HierarchyStore for SettableRows {
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

/// Every agent runs and is ready unless a test says otherwise.
#[derive(Default)]
struct Targets {
    stopped: Mutex<Option<AgentId>>,
    busy: Mutex<bool>,
}

impl DirectiveTarget for Targets {
    fn is_running(&self, agent_id: AgentId) -> Result<bool, String> {
        Ok(*locked(&self.stopped) != Some(agent_id))
    }

    fn name_of(&self, agent_id: AgentId) -> Option<String> {
        Some(format!(
            "agent-{}",
            agent_id.to_string().get(..8).unwrap_or("?")
        ))
    }

    fn readiness(&self, _agent_id: AgentId) -> Result<Readiness, String> {
        Ok(if *locked(&self.busy) {
            Readiness::Busy
        } else {
            Readiness::Ready
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Entry {
    Accepted {
        sender: AgentId,
        receiver: AgentId,
        text: String,
    },
    Refused {
        sender: AgentId,
        parent: Option<AgentId>,
        reason: String,
    },
}

/// Records every port call a test reads back.
#[derive(Default)]
struct Recorder {
    entries: Mutex<Vec<Entry>>,
    delivered: Mutex<Vec<(AgentId, ReportKind, String)>>,
    typed: Mutex<Vec<(AgentId, String)>>,
    received: Mutex<Vec<AgentId>>,
    directives: Mutex<Vec<(AgentId, Option<String>)>>,
}

impl ReportJournal for Recorder {
    fn accepted(&self, sender: AgentId, receiver: AgentId, record_text: &str) {
        locked(&self.entries).push(Entry::Accepted {
            sender,
            receiver,
            text: record_text.to_owned(),
        });
    }

    fn refused(&self, sender: AgentId, parent: Option<AgentId>, _text: &str, reason: &str) {
        locked(&self.entries).push(Entry::Refused {
            sender,
            parent,
            reason: reason.to_owned(),
        });
    }
}

impl PersonReports for Recorder {
    fn deliver(&self, sender: AgentId, kind: ReportKind, text: &str) {
        locked(&self.delivered).push((sender, kind, text.to_owned()));
    }
}

impl DirectiveWriter for Recorder {
    fn deliver(&self, agent_id: AgentId, text: &str) -> Result<Delivery, String> {
        locked(&self.typed).push((agent_id, text.to_owned()));
        Ok(Delivery::Submitted)
    }
}

impl DirectiveReceipt for Recorder {
    fn received(&self, target: AgentId, _text: &str) {
        locked(&self.received).push(target);
    }
}

impl DirectiveJournal for Recorder {
    fn record(&self, sender: AgentId, _target: AgentId, _text: &str, failure: Option<&str>) {
        locked(&self.directives).push((sender, failure.map(str::to_owned)));
    }
}

struct NoHoldStore;

impl HoldStore for NoHoldStore {
    fn load_holds(&self) -> Result<Vec<Hold>, HierarchyApplicationError> {
        Ok(Vec::new())
    }

    fn save_hold(&self, _hold: &Hold) -> Result<(), HierarchyApplicationError> {
        Ok(())
    }

    fn delete_hold(&self, _scope: HoldScope) -> Result<(), HierarchyApplicationError> {
        Ok(())
    }

    fn delete_agent_holds(&self, _agent_ids: &[AgentId]) -> Result<(), HierarchyApplicationError> {
        Ok(())
    }
}

struct DefaultLimits;

impl RateLimitSource for DefaultLimits {
    fn limits(&self) -> RateLimits {
        RateLimits::default()
    }
}

/// One clock for the holds and the queue; tests do not move it, so the
/// settle gap after a typing is never over and the queue keeps its order.
struct FixedClock;

impl HoldClock for FixedClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_unix_millis(NOW_MILLIS)
    }

    fn steady_now(&self) -> Timestamp {
        Timestamp::from_unix_millis(NOW_MILLIS)
    }
}

impl DirectiveClock for FixedClock {
    fn steady_now(&self) -> Timestamp {
        Timestamp::from_unix_millis(NOW_MILLIS)
    }
}

struct Quiet;

impl HoldJournal for Quiet {
    fn held(&self, _hold: &Hold) {}
    fn released(&self, _scope: HoldScope) {}
    fn storage_failed(&self, _message: &str) {}
}

impl HoldNotifier for Quiet {
    fn auto_held(&self, _agent: AgentId, _limit: &LimitRefusal) {}
    fn resume_held(&self, _agent: AgentId, _reason: &str) {}
}

struct Harness {
    handler: SendReportHandler,
    directives: Arc<SendDirectiveHandler>,
    recorder: Arc<Recorder>,
    targets: Arc<Targets>,
    store: Arc<SettableRows>,
    holds: Arc<HoldService>,
}

fn harness_with(rows: Vec<TreeRow>) -> Harness {
    let recorder = Arc::new(Recorder::default());
    let targets = Arc::new(Targets::default());
    let store = Arc::new(SettableRows {
        rows: Mutex::new(rows),
        tree_writes: Mutex::new(()),
    });
    let holds = Arc::new(
        HoldService::new(
            Arc::new(NoHoldStore),
            Arc::new(DefaultLimits),
            Arc::new(Quiet),
            Arc::new(Quiet),
            Arc::new(FixedClock),
        )
        .expect("hold service"),
    );
    let directives = Arc::new(
        SendDirectiveHandler::new(DirectivePorts {
            store: store.clone(),
            targets: targets.clone(),
            writer: recorder.clone(),
            journal: recorder.clone(),
            receipt: recorder.clone(),
            clock: Arc::new(FixedClock),
        })
        .with_hold_guard(Arc::clone(&holds))
        .with_report_journal(recorder.clone()),
    );
    let handler = SendReportHandler::new(ReportPorts {
        store: store.clone(),
        targets: targets.clone(),
        journal: recorder.clone(),
        person: recorder.clone(),
        parent_delivery: Arc::clone(&directives),
    })
    .with_hold_guard(Arc::clone(&holds));
    Harness {
        handler,
        directives,
        recorder,
        targets,
        store,
        holds,
    }
}

fn harness() -> Harness {
    harness_with(company())
}

fn report(sender: u128, text: &str, for_person: bool) -> Report {
    Report {
        sender: id(sender),
        kind: ReportKind::Question,
        text: text.to_owned(),
        for_person,
    }
}

fn refusal_reasons(recorder: &Recorder) -> Vec<String> {
    locked(&recorder.entries)
        .iter()
        .filter_map(|entry| match entry {
            Entry::Refused { reason, .. } => Some(reason.clone()),
            Entry::Accepted { .. } => None,
        })
        .collect()
}

#[test]
fn the_ceo_reaches_the_person_with_the_text_in_visible_form() {
    let harness = harness();

    let delivery = harness
        .handler
        .handle(&report(CEO, "pick\nA \u{1b}[31m or \u{202e}B", false));

    assert_eq!(delivery, Ok(ReportDelivery::NeedsYou));
    let visible = "pick␊A ␛[31m or \\u202EB".to_owned();
    assert_eq!(
        *locked(&harness.recorder.delivered),
        vec![(id(CEO), ReportKind::Question, visible.clone())]
    );
    assert_eq!(
        *locked(&harness.recorder.entries),
        vec![Entry::Accepted {
            sender: id(CEO),
            receiver: id(CEO),
            text: format!("question: {visible}"),
        }]
    );
}

#[test]
fn a_member_reaches_the_person_when_it_asks_for_the_person() {
    let harness = harness();

    let delivery = harness.handler.handle(&report(MEMBER, "a key?", true));

    assert_eq!(delivery, Ok(ReportDelivery::NeedsYou));
    assert_eq!(locked(&harness.recorder.delivered).len(), 1);
    assert!(locked(&harness.recorder.typed).is_empty());
}

#[test]
fn a_report_to_a_ready_parent_is_typed_as_one_line_and_sets_no_task_text() {
    let harness = harness();

    let delivery = harness.handler.handle(&report(MEMBER, "done?\nyes", false));

    assert_eq!(
        delivery,
        Ok(ReportDelivery::Parent(DirectiveOutcome::Submitted))
    );
    let typed = locked(&harness.recorder.typed).clone();
    assert_eq!(typed.len(), 1);
    assert_eq!(typed[0].0, id(LEAD));
    assert!(
        typed[0].1.starts_with("[report from agent-"),
        "{}",
        typed[0].1
    );
    assert!(
        typed[0]
            .1
            .ends_with(&format!("({}): question] done?␊yes", id(MEMBER))),
        "{}",
        typed[0].1
    );
    assert!(locked(&harness.recorder.received).is_empty());
    assert!(
        locked(&harness.recorder.entries).contains(&Entry::Accepted {
            sender: id(MEMBER),
            receiver: id(LEAD),
            text: "question: done?␊yes".to_owned(),
        })
    );
}

#[test]
fn a_report_to_a_busy_parent_waits_and_a_second_one_replaces_it_in_place() {
    let harness = harness();
    *locked(&harness.targets.busy) = true;

    let first = harness.handler.handle(&report(MEMBER, "first", false));
    let second = harness.handler.handle(&report(MEMBER, "second", false));

    let waiting_first = Ok(ReportDelivery::Parent(DirectiveOutcome::Waiting {
        position: 1,
    }));
    assert_eq!(first, waiting_first);
    assert_eq!(second, waiting_first);
    assert!(locked(&harness.recorder.typed).is_empty());
    assert!(
        refusal_reasons(&harness.recorder)
            .contains(&"withdrawn before typing: replaced by a newer report".to_owned())
    );

    *locked(&harness.targets.busy) = false;
    harness.directives.deliver_waiting();

    let typed = locked(&harness.recorder.typed).clone();
    assert_eq!(typed.len(), 1);
    assert!(typed[0].1.ends_with("question] second"), "{}", typed[0].1);
    assert!(locked(&harness.recorder.received).is_empty());
}

#[test]
fn ten_waiting_reports_leave_room_for_a_directive_from_above_and_refuse_an_eleventh() {
    let extra_members = u128::try_from(REPORT_SHARE).expect("small share") + 1;
    let mut rows = company();
    rows.extend(
        (0..extra_members).map(|index| row(FIRST_EXTRA_MEMBER + index, Role::Member, Some(LEAD))),
    );
    let harness = harness_with(rows);
    *locked(&harness.targets.busy) = true;
    for index in 0..extra_members - 1 {
        let accepted = harness
            .handler
            .handle(&report(FIRST_EXTRA_MEMBER + index, "done", false));
        assert!(
            matches!(accepted, Ok(ReportDelivery::Parent(_))),
            "{accepted:?}"
        );
    }
    let last_member = FIRST_EXTRA_MEMBER + extra_members - 1;

    let refused = harness.handler.handle(&report(last_member, "late", false));
    let directive = harness.directives.handle(id(CEO), id(LEAD), "next task");

    let Err(HierarchyApplicationError::Refused(reason)) = refused else {
        panic!("expected a no-room refusal, got {refused:?}");
    };
    assert!(reason.contains("no room for reports"), "{reason}");
    assert_eq!(
        directive,
        Ok(DirectiveOutcome::Waiting {
            position: REPORT_SHARE + 1
        })
    );
}

#[test]
fn a_no_room_refusal_takes_no_slot_of_the_reports_limit() {
    let extra_members = u128::try_from(REPORT_SHARE).expect("small share") + 1;
    let mut rows = company();
    rows.extend(
        (0..extra_members).map(|index| row(FIRST_EXTRA_MEMBER + index, Role::Member, Some(LEAD))),
    );
    let harness = harness_with(rows);
    *locked(&harness.targets.busy) = true;
    for index in 0..extra_members - 1 {
        harness
            .handler
            .handle(&report(FIRST_EXTRA_MEMBER + index, "done", false))
            .expect("room");
    }
    let last_member = FIRST_EXTRA_MEMBER + extra_members - 1;
    for _ in 0..=RateLimits::default().report {
        assert!(
            harness
                .handler
                .handle(&report(last_member, "late", false))
                .is_err()
        );
    }

    let for_person = harness.handler.handle(&report(last_member, "help", true));

    assert_eq!(for_person, Ok(ReportDelivery::NeedsYou));
}

#[test]
fn a_waiting_report_of_a_moved_sender_is_withdrawn_untyped() {
    let harness = harness();
    *locked(&harness.targets.busy) = true;
    harness
        .handler
        .handle(&report(MEMBER, "status", false))
        .expect("waits");
    for row in locked(&harness.store.rows).iter_mut() {
        if row.agent_id == id(MEMBER) {
            row.parent_agent_id = Some(id(OTHER_LEAD));
        }
    }

    *locked(&harness.targets.busy) = false;
    harness.directives.deliver_waiting();

    assert!(locked(&harness.recorder.typed).is_empty());
    assert!(
        refusal_reasons(&harness.recorder)
            .iter()
            .any(|reason| reason
                .starts_with("withdrawn before typing: the sender is no longer a child")),
        "{:?}",
        refusal_reasons(&harness.recorder)
    );
}

#[test]
fn a_report_to_a_stopped_parent_is_refused_in_both_records() {
    let harness = harness();
    *locked(&harness.targets.stopped) = Some(id(LEAD));

    let refused = harness.handler.handle(&report(MEMBER, "done", false));

    assert!(matches!(
        refused,
        Err(HierarchyApplicationError::Invalid(
            HierarchyError::NotRunning { .. }
        ))
    ));
    assert!(matches!(
        locked(&harness.recorder.entries).as_slice(),
        [Entry::Refused { sender, parent: Some(parent), .. }]
            if *sender == id(MEMBER) && *parent == id(LEAD)
    ));
}

#[test]
fn a_blank_or_long_report_is_refused_and_takes_no_limit_slot() {
    let harness = harness();
    let long = "x".repeat(hierarchy_domain::REPORT_MAX_CHARS + 1);

    let blank = harness.handler.handle(&report(CEO, "  ", false));
    let too_long = harness.handler.handle(&report(CEO, &long, false));

    assert!(matches!(
        blank,
        Err(HierarchyApplicationError::Invalid(
            HierarchyError::Empty { .. }
        ))
    ));
    assert!(matches!(
        too_long,
        Err(HierarchyApplicationError::Invalid(
            HierarchyError::TooLong { .. }
        ))
    ));
    for _ in 0..RateLimits::default().report {
        assert_eq!(
            harness.handler.handle(&report(CEO, "ok", false)),
            Ok(ReportDelivery::NeedsYou)
        );
    }
}

#[test]
fn the_report_past_the_limit_is_refused_with_the_limit_name() {
    let harness = harness();
    for _ in 0..RateLimits::default().report {
        harness
            .handler
            .handle(&report(MEMBER, "step", false))
            .expect("within the limit");
    }

    let refused = harness.handler.handle(&report(MEMBER, "one more", true));

    let Err(HierarchyApplicationError::Refused(reason)) = refused else {
        panic!("expected a limit refusal, got {refused:?}");
    };
    assert!(reason.contains("reports sent by one agent"), "{reason}");
}

#[test]
fn an_office_hold_refuses_a_report_to_the_person_and_to_a_parent() {
    let harness = harness();
    harness
        .holds
        .set_hold(HoldScope::Office)
        .expect("office hold");

    let to_person = harness.handler.handle(&report(CEO, "status", false));
    let to_parent = harness.handler.handle(&report(MEMBER, "status", false));

    assert!(matches!(
        to_person,
        Err(HierarchyApplicationError::Refused(_))
    ));
    assert!(matches!(
        to_parent,
        Err(HierarchyApplicationError::Refused(_))
    ));
    assert!(locked(&harness.recorder.delivered).is_empty());
    assert!(locked(&harness.recorder.typed).is_empty());
}

#[test]
fn a_hold_on_the_parent_refuses_its_childs_report() {
    let harness = harness();
    harness
        .holds
        .set_hold(HoldScope::Agent(id(LEAD)))
        .expect("lead hold");

    let refused = harness.handler.handle(&report(MEMBER, "a key?", true));

    assert!(matches!(
        refused,
        Err(HierarchyApplicationError::Refused(_))
    ));
}

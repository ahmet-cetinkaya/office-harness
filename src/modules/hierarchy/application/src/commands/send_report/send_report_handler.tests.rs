// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Mutex, MutexGuard, PoisonError};

use hierarchy_domain::{Hold, HoldScope, LimitRefusal, RateLimits, Role, TreeRow};
use kernel::Timestamp;

use super::*;
use crate::common::interfaces::{HoldStore, TreeExclusion, TreeSnapshot};
use crate::common::ports::{HoldClock, HoldJournal, HoldNotifier, RateLimitSource};

const CEO: u128 = 1;
const LEAD: u128 = 2;
const MEMBER: u128 = 3;
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

struct FixedRows {
    rows: Vec<TreeRow>,
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

/// Every agent runs unless it is the one named as stopped.
struct Targets {
    stopped: Option<AgentId>,
}

impl DirectiveTarget for Targets {
    fn is_running(&self, agent_id: AgentId) -> Result<bool, String> {
        Ok(self.stopped != Some(agent_id))
    }

    fn name_of(&self, agent_id: AgentId) -> Option<String> {
        Some(format!("agent {agent_id}"))
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

#[derive(Default)]
struct Recorder {
    entries: Mutex<Vec<Entry>>,
    delivered: Mutex<Vec<(AgentId, ReportKind, String)>>,
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

struct FixedClock;

impl HoldClock for FixedClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_unix_millis(NOW_MILLIS)
    }

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
    recorder: Arc<Recorder>,
    holds: Arc<HoldService>,
}

fn harness(stopped: Option<u128>) -> Harness {
    let recorder = Arc::new(Recorder::default());
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
    let store = Arc::new(FixedRows {
        rows: vec![
            row(CEO, Role::Ceo, None),
            row(LEAD, Role::Lead, Some(CEO)),
            row(MEMBER, Role::Member, Some(LEAD)),
        ],
        tree_writes: Mutex::new(()),
    });
    let handler = SendReportHandler::new(
        store,
        Arc::new(Targets {
            stopped: stopped.map(id),
        }),
        recorder.clone(),
        recorder.clone(),
    )
    .with_hold_guard(Arc::clone(&holds));
    Harness {
        handler,
        recorder,
        holds,
    }
}

fn report(sender: u128, text: &str, for_person: bool) -> Report {
    Report {
        sender: id(sender),
        kind: ReportKind::Question,
        text: text.to_owned(),
        for_person,
    }
}

#[test]
fn the_ceo_reaches_the_person_with_the_text_in_visible_form() {
    let harness = harness(None);

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
    let harness = harness(None);

    let delivery = harness.handler.handle(&report(MEMBER, "a key?", true));

    assert_eq!(delivery, Ok(ReportDelivery::NeedsYou));
    assert_eq!(locked(&harness.recorder.delivered).len(), 1);
}

#[test]
fn a_report_to_a_stopped_parent_is_refused_in_both_records() {
    let harness = harness(Some(LEAD));

    let refused = harness.handler.handle(&report(MEMBER, "done", false));

    assert!(matches!(
        refused,
        Err(HierarchyApplicationError::Invalid(
            HierarchyError::NotRunning { .. }
        ))
    ));
    assert!(matches!(
        locked(&harness.recorder.entries).as_slice(),
        [Entry::Refused { sender, parent: Some(parent), .. }] if *sender == id(MEMBER) && *parent == id(LEAD)
    ));
    assert!(locked(&harness.recorder.delivered).is_empty());
}

#[test]
fn a_blank_or_long_report_is_refused_and_takes_no_limit_slot() {
    let harness = harness(None);
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
    let harness = harness(None);
    for _ in 0..RateLimits::default().report {
        harness
            .handler
            .handle(&report(CEO, "step", false))
            .expect("within the limit");
    }

    let refused = harness.handler.handle(&report(CEO, "one more", false));

    let Err(HierarchyApplicationError::Refused(reason)) = refused else {
        panic!("expected a limit refusal, got {refused:?}");
    };
    assert!(reason.contains("reports sent by one agent"), "{reason}");
}

#[test]
fn an_office_hold_refuses_a_report_for_the_person() {
    let harness = harness(None);
    harness
        .holds
        .set_hold(HoldScope::Office)
        .expect("office hold");

    let refused = harness.handler.handle(&report(CEO, "status", false));

    assert!(matches!(
        refused,
        Err(HierarchyApplicationError::Refused(_))
    ));
    assert!(locked(&harness.recorder.delivered).is_empty());
}

#[test]
fn a_hold_on_the_parent_refuses_its_childs_report() {
    let harness = harness(None);
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

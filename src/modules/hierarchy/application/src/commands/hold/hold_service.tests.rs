// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

use super::*;
use hierarchy_domain::{AUTO_HOLD_REFUSAL_COUNT, RateLimits, Role};
use kernel::Timestamp;

const CEO: u128 = 1;
const LEAD: u128 = 2;
const MEMBER: u128 = 3;
const START_MILLIS: i64 = 1_700_000_000_000;
const SPREAD_MILLIS: i64 = 1000;
const SMALL_DIRECTIVE_LIMIT: u32 = 2;

fn id(value: u128) -> AgentId {
    AgentId::from_u128(value)
}

fn row(agent: u128, parent: Option<u128>) -> TreeRow {
    TreeRow {
        agent_id: id(agent),
        name: agent.to_string(),
        role: Some(Role::Member),
        parent_agent_id: parent.map(id),
        team_id: None,
    }
}

fn rows() -> Vec<TreeRow> {
    vec![
        row(CEO, None),
        row(LEAD, Some(CEO)),
        row(MEMBER, Some(LEAD)),
    ]
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[derive(Default)]
struct MemoryStore {
    holds: Mutex<Vec<Hold>>,
    fail_writes: AtomicBool,
}

impl MemoryStore {
    fn check_write(&self) -> Result<(), HierarchyApplicationError> {
        if self.fail_writes.load(Ordering::SeqCst) {
            return Err(HierarchyApplicationError::Storage("disk full".to_owned()));
        }
        Ok(())
    }
}

impl HoldStore for MemoryStore {
    fn load_holds(&self) -> Result<Vec<Hold>, HierarchyApplicationError> {
        Ok(locked(&self.holds).clone())
    }

    fn save_hold(&self, hold: &Hold) -> Result<(), HierarchyApplicationError> {
        self.check_write()?;
        locked(&self.holds).push(hold.clone());
        Ok(())
    }

    fn delete_hold(&self, scope: HoldScope) -> Result<(), HierarchyApplicationError> {
        self.check_write()?;
        locked(&self.holds).retain(|hold| hold.scope != scope);
        Ok(())
    }

    fn delete_agent_holds(&self, agent_ids: &[AgentId]) -> Result<(), HierarchyApplicationError> {
        self.check_write()?;
        locked(&self.holds).retain(|hold| match hold.scope {
            HoldScope::Agent(agent_id) => !agent_ids.contains(&agent_id),
            HoldScope::Office => true,
        });
        Ok(())
    }
}

struct SettableLimits(Mutex<RateLimits>);

impl RateLimitSource for SettableLimits {
    fn limits(&self) -> RateLimits {
        *locked(&self.0)
    }
}

struct SettableClock(AtomicI64);

impl HoldClock for SettableClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_unix_millis(self.0.load(Ordering::SeqCst))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    Held(HoldScope, HoldSetter),
    Released(HoldScope),
    Refused(AgentId, String),
    Notified(AgentId),
}

#[derive(Default)]
struct Recorder(Mutex<Vec<Event>>);

impl Recorder {
    fn push(&self, event: Event) {
        locked(&self.0).push(event);
    }

    fn events(&self) -> Vec<Event> {
        locked(&self.0).clone()
    }

    fn count(&self, wanted: fn(&Event) -> bool) -> usize {
        self.events().iter().filter(|event| wanted(event)).count()
    }
}

impl HoldJournal for Recorder {
    fn held(&self, hold: &Hold) {
        self.push(Event::Held(hold.scope, hold.set_by));
    }

    fn released(&self, scope: HoldScope) {
        self.push(Event::Released(scope));
    }

    fn refused(&self, caller: AgentId, reason: &str) {
        self.push(Event::Refused(caller, reason.to_owned()));
    }
}

impl HoldNotifier for Recorder {
    fn auto_held(&self, agent: AgentId, _limit: &LimitRefusal) {
        self.push(Event::Notified(agent));
    }
}

struct Fixture {
    service: HoldService,
    store: Arc<MemoryStore>,
    limits: Arc<SettableLimits>,
    clock: Arc<SettableClock>,
    recorder: Arc<Recorder>,
}

fn fixture_with(stored: Vec<Hold>) -> Fixture {
    let store = Arc::new(MemoryStore {
        holds: Mutex::new(stored),
        fail_writes: AtomicBool::new(false),
    });
    let limits = Arc::new(SettableLimits(Mutex::new(RateLimits::default())));
    let clock = Arc::new(SettableClock(AtomicI64::new(START_MILLIS)));
    let recorder = Arc::new(Recorder::default());
    let service = HoldService::new(
        Arc::clone(&store) as Arc<dyn HoldStore>,
        Arc::clone(&limits) as Arc<dyn RateLimitSource>,
        Arc::clone(&recorder) as Arc<dyn HoldJournal>,
        Arc::clone(&recorder) as Arc<dyn HoldNotifier>,
        Arc::clone(&clock) as Arc<dyn HoldClock>,
    )
    .expect("service");
    Fixture {
        service,
        store,
        limits,
        clock,
        recorder,
    }
}

fn fixture() -> Fixture {
    fixture_with(Vec::new())
}

impl Fixture {
    fn fail_writes(&self) {
        self.store.fail_writes.store(true, Ordering::SeqCst);
    }

    fn set_directive_limit(&self, allowed: u32) {
        locked(&self.limits.0).directive = allowed;
    }

    fn advance(&self, millis: i64) {
        self.clock.0.fetch_add(millis, Ordering::SeqCst);
    }

    fn directive(&self, caller: u128, target: u128) -> Result<(), GuardRefusal> {
        self.service.guard(
            id(caller),
            AgentAction::Directive { target: id(target) },
            Some(id(target)),
            &rows(),
        )
    }

    fn lifecycle(&self, caller: u128) -> Result<(), GuardRefusal> {
        self.service
            .guard(id(caller), AgentAction::Lifecycle, None, &rows())
    }

    /// Spends the lifecycle allowance, then returns the refusals after it.
    fn exhaust_lifecycle(&self, caller: u128) {
        for _ in 0..RateLimits::default().lifecycle {
            self.lifecycle(caller).expect("within the limit");
        }
    }
}

fn is_notification(event: &Event) -> bool {
    matches!(event, Event::Notified(_))
}

fn is_held(event: &Event) -> bool {
    matches!(event, Event::Held(..))
}

fn is_refusal(event: &Event) -> bool {
    matches!(event, Event::Refused(..))
}

fn conflict_text(error: HierarchyApplicationError) -> String {
    match error {
        HierarchyApplicationError::Conflict(text) => text,
        other => panic!("expected a conflict, got {other:?}"),
    }
}

#[test]
fn setting_and_releasing_the_office_hold_journals_each_change() {
    let fixture = fixture();
    let view = fixture.service.set_hold(HoldScope::Office).expect("set");
    assert!(view.office.is_some());
    assert_eq!(locked(&fixture.store.holds).len(), 1);
    let view = fixture
        .service
        .release_hold(HoldScope::Office)
        .expect("release");
    assert_eq!(view, HoldsView::default());
    assert_eq!(locked(&fixture.store.holds).len(), 0);
    assert_eq!(
        fixture.recorder.events(),
        [
            Event::Held(HoldScope::Office, HoldSetter::User),
            Event::Released(HoldScope::Office)
        ]
    );
}

#[test]
fn holding_twice_is_a_conflict_with_the_scope_message() {
    let fixture = fixture();
    fixture.service.set_hold(HoldScope::Office).expect("set");
    let agent = HoldScope::Agent(id(MEMBER));
    fixture.service.set_hold(agent).expect("set agent");
    assert_eq!(
        conflict_text(fixture.service.set_hold(HoldScope::Office).unwrap_err()),
        "the office is already on hold"
    );
    assert_eq!(
        conflict_text(fixture.service.set_hold(agent).unwrap_err()),
        "the agent is already on hold"
    );
}

#[test]
fn releasing_what_is_not_held_is_a_conflict() {
    let fixture = fixture();
    assert_eq!(
        conflict_text(fixture.service.release_hold(HoldScope::Office).unwrap_err()),
        "the office is not on hold"
    );
    assert_eq!(
        conflict_text(
            fixture
                .service
                .release_hold(HoldScope::Agent(id(MEMBER)))
                .unwrap_err()
        ),
        "the agent is not on hold"
    );
    assert_eq!(fixture.recorder.events().len(), 0);
}

#[test]
fn a_failed_write_leaves_the_holds_and_the_journal_unchanged() {
    let fixture = fixture();
    fixture.service.set_hold(HoldScope::Office).expect("set");
    fixture.fail_writes();
    assert!(matches!(
        fixture.service.set_hold(HoldScope::Agent(id(MEMBER))),
        Err(HierarchyApplicationError::Storage(_))
    ));
    assert!(matches!(
        fixture.service.release_hold(HoldScope::Office),
        Err(HierarchyApplicationError::Storage(_))
    ));
    let view = fixture.service.list();
    assert!(view.office.is_some());
    assert_eq!(view.agents.len(), 0);
    assert_eq!(fixture.recorder.events().len(), 1);
}

#[test]
fn releasing_the_office_keeps_an_automatic_agent_hold() {
    let fixture = fixture();
    fixture.exhaust_lifecycle(MEMBER);
    for _ in 0..AUTO_HOLD_REFUSAL_COUNT {
        let _ = fixture.lifecycle(MEMBER);
    }
    fixture.service.set_hold(HoldScope::Office).expect("set");
    let view = fixture
        .service
        .release_hold(HoldScope::Office)
        .expect("release");
    assert_eq!(view.agents.len(), 1);
    assert_eq!(view.agents[0].set_by, HoldSetter::Office);
}

#[test]
fn an_office_hold_refusal_beats_the_limit_and_is_journaled() {
    let fixture = fixture();
    fixture.service.set_hold(HoldScope::Office).expect("set");
    let refusal = fixture.lifecycle(MEMBER).unwrap_err();
    assert_eq!(refusal.reason, "the office is on hold");
    assert!(!refusal.holds_changed);
    assert_eq!(
        fixture.recorder.events().last(),
        Some(&Event::Refused(id(MEMBER), refusal.reason))
    );
}

#[test]
fn an_agent_hold_covers_a_held_target_and_a_held_ancestor() {
    let fixture = fixture();
    fixture
        .service
        .set_hold(HoldScope::Agent(id(MEMBER)))
        .expect("set");
    assert!(fixture.directive(LEAD, MEMBER).is_err());
    assert!(fixture.lifecycle(MEMBER).is_err());
    assert!(fixture.lifecycle(LEAD).is_ok());
}

#[test]
fn hold_refusals_neither_count_in_windows_nor_feed_the_tracker() {
    let fixture = fixture();
    fixture
        .service
        .set_hold(HoldScope::Agent(id(MEMBER)))
        .expect("set");
    for _ in 0..RateLimits::default().lifecycle * 2 {
        assert!(fixture.lifecycle(MEMBER).is_err());
    }
    fixture
        .service
        .release_hold(HoldScope::Agent(id(MEMBER)))
        .expect("release");
    // A full allowance is still free, and no automatic hold happened.
    fixture.exhaust_lifecycle(MEMBER);
    assert_eq!(fixture.recorder.count(is_notification), 0);
}

#[test]
fn the_exact_limit_passes_and_the_next_action_is_refused() {
    let fixture = fixture();
    fixture.exhaust_lifecycle(MEMBER);
    let refusal = fixture.lifecycle(MEMBER).unwrap_err();
    assert!(refusal.reason.contains("allows 10 per 60 seconds"));
    assert!(!refusal.holds_changed);
}

#[test]
fn the_third_limit_refusal_holds_the_caller_once_and_notifies_once() {
    let fixture = fixture();
    fixture.exhaust_lifecycle(MEMBER);
    for _ in 1..AUTO_HOLD_REFUSAL_COUNT {
        assert!(!fixture.lifecycle(MEMBER).unwrap_err().holds_changed);
    }
    let third = fixture.lifecycle(MEMBER).unwrap_err();
    assert!(third.holds_changed);
    assert!(third.reason.contains("allows 10 per 60 seconds"));
    let held = fixture.service.list().agents;
    assert_eq!(held.len(), 1);
    assert_eq!(held[0].scope, HoldScope::Agent(id(MEMBER)));
    assert_eq!(held[0].set_by, HoldSetter::Office);
    assert_eq!(held[0].reason.as_deref(), Some(third.reason.as_str()));
    let events = fixture.recorder.events();
    assert_eq!(
        events[events.len() - 2..],
        [
            Event::Held(HoldScope::Agent(id(MEMBER)), HoldSetter::Office),
            Event::Notified(id(MEMBER)),
        ]
    );

    for _ in 0..2 {
        let later = fixture.lifecycle(MEMBER).unwrap_err();
        assert!(later.reason.contains("on hold"));
        assert!(!later.holds_changed);
    }
    assert_eq!(fixture.recorder.count(is_notification), 1);
    assert_eq!(fixture.recorder.count(is_held), 1);
    assert_eq!(fixture.service.list().agents.len(), 1);
}

#[test]
fn a_failed_automatic_hold_write_keeps_the_refusal_and_holds_nothing() {
    let fixture = fixture();
    fixture.exhaust_lifecycle(MEMBER);
    fixture.fail_writes();
    let mut last = None;
    for _ in 0..AUTO_HOLD_REFUSAL_COUNT {
        last = Some(fixture.lifecycle(MEMBER).unwrap_err());
    }
    let last = last.expect("refusals");
    assert!(!last.holds_changed);
    assert!(last.reason.contains("allows"));
    assert_eq!(fixture.service.list().agents.len(), 0);
    assert_eq!(fixture.recorder.count(is_notification), 0);
    assert_eq!(fixture.recorder.count(is_held), 0);
    assert_eq!(fixture.recorder.count(is_refusal), AUTO_HOLD_REFUSAL_COUNT);
}

#[test]
fn an_agent_already_held_gets_no_second_hold_or_notification() {
    let fixture = fixture();
    fixture.exhaust_lifecycle(LEAD);
    fixture
        .service
        .set_hold(HoldScope::Agent(id(LEAD)))
        .expect("set");
    for _ in 0..AUTO_HOLD_REFUSAL_COUNT {
        assert!(fixture.lifecycle(LEAD).is_err());
    }
    assert_eq!(fixture.service.list().agents.len(), 1);
    assert_eq!(fixture.recorder.count(is_notification), 0);
}

#[test]
fn limit_refusals_spread_past_the_window_do_not_hold_the_caller() {
    let fixture = fixture();
    fixture.exhaust_lifecycle(MEMBER);
    for _ in 0..AUTO_HOLD_REFUSAL_COUNT {
        // Fresh allowance each round: the old takes age out with the clock.
        fixture.advance(RATE_WINDOW_STEP_MILLIS);
        fixture.exhaust_lifecycle(MEMBER);
        assert!(!fixture.lifecycle(MEMBER).unwrap_err().holds_changed);
    }
    assert_eq!(fixture.service.list().agents.len(), 0);
}

const RATE_WINDOW_STEP_MILLIS: i64 = 61 * SPREAD_MILLIS;

#[test]
fn limits_are_read_on_every_call() {
    let fixture = fixture();
    for _ in 0..=SMALL_DIRECTIVE_LIMIT {
        fixture
            .directive(LEAD, MEMBER)
            .expect("default limit allows it");
    }
    fixture.advance(RATE_WINDOW_STEP_MILLIS);
    fixture.set_directive_limit(SMALL_DIRECTIVE_LIMIT);
    fixture.directive(LEAD, MEMBER).expect("first");
    fixture.directive(LEAD, MEMBER).expect("second");
    assert!(fixture.directive(LEAD, MEMBER).is_err());
}

#[test]
fn forget_agents_drops_only_those_agents_holds() {
    let fixture = fixture();
    fixture.service.set_hold(HoldScope::Office).expect("office");
    fixture
        .service
        .set_hold(HoldScope::Agent(id(LEAD)))
        .expect("lead");
    fixture
        .service
        .set_hold(HoldScope::Agent(id(MEMBER)))
        .expect("member");
    fixture
        .service
        .forget_agents(&[id(LEAD)])
        .expect("forgotten");
    let view = fixture.service.list();
    assert!(view.office.is_some());
    assert_eq!(view.agents.len(), 1);
    assert_eq!(view.agents[0].scope, HoldScope::Agent(id(MEMBER)));
    assert_eq!(locked(&fixture.store.holds).len(), 2);
}

#[test]
fn a_failed_forget_keeps_the_holds() {
    let fixture = fixture();
    fixture
        .service
        .set_hold(HoldScope::Agent(id(LEAD)))
        .expect("lead");
    fixture.fail_writes();
    assert!(fixture.service.forget_agents(&[id(LEAD)]).is_err());
    assert_eq!(fixture.service.list().agents.len(), 1);
}

#[test]
fn prune_drops_holds_of_agents_that_no_longer_exist() {
    let stored = vec![
        Hold {
            scope: HoldScope::Agent(id(LEAD)),
            set_by: HoldSetter::User,
            since: Timestamp::from_unix_millis(START_MILLIS),
            reason: None,
        },
        Hold {
            scope: HoldScope::Agent(id(MEMBER)),
            set_by: HoldSetter::User,
            since: Timestamp::from_unix_millis(START_MILLIS),
            reason: None,
        },
    ];
    let fixture = fixture_with(stored);
    fixture
        .service
        .prune(&HashSet::from([id(MEMBER)]))
        .expect("pruned");
    let view = fixture.service.list();
    assert_eq!(view.agents.len(), 1);
    assert_eq!(view.agents[0].scope, HoldScope::Agent(id(MEMBER)));
    assert_eq!(locked(&fixture.store.holds).len(), 1);
}

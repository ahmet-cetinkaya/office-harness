// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

const SENDER: u128 = 1;
const TARGET: u128 = 2;
const OTHER_TARGET: u128 = 3;

fn id(value: u128) -> AgentId {
    AgentId::from_u128(value)
}

fn at_seconds(seconds: i64) -> Timestamp {
    Timestamp::from_unix_millis(seconds * MILLIS_PER_SECOND)
}

fn directive_to(target: u128) -> AgentAction {
    AgentAction::Directive { target: id(target) }
}

/// Limits wide enough that only the kind under test can refuse.
fn wide_limits() -> RateLimits {
    RateLimits {
        directive: 100,
        directive_target: 100,
        create: 100,
        lifecycle: 100,
    }
}

fn assert_passes_limit_then_refuses(
    action: AgentAction,
    limits: &RateLimits,
    kind: LimitKind,
    allowed: u32,
) {
    let mut windows = RateWindows::default();
    for _ in 0..allowed {
        assert_eq!(
            windows.try_take(id(SENDER), action, limits, at_seconds(0)),
            Ok(())
        );
    }
    assert_eq!(
        windows.try_take(id(SENDER), action, limits, at_seconds(0)),
        Err(LimitRefusal { kind, allowed })
    );
}

#[test]
fn exactly_the_directive_limit_passes_and_the_next_is_refused() {
    let limits = RateLimits {
        directive: 4,
        ..wide_limits()
    };
    assert_passes_limit_then_refuses(directive_to(TARGET), &limits, LimitKind::Directive, 4);
}

#[test]
fn exactly_the_target_limit_passes_and_the_next_is_refused() {
    let limits = RateLimits {
        directive_target: 2,
        ..wide_limits()
    };
    assert_passes_limit_then_refuses(directive_to(TARGET), &limits, LimitKind::DirectiveTarget, 2);
}

#[test]
fn exactly_the_create_limit_passes_and_the_next_is_refused() {
    let limits = RateLimits {
        create: 5,
        ..wide_limits()
    };
    assert_passes_limit_then_refuses(AgentAction::Create, &limits, LimitKind::Create, 5);
}

#[test]
fn exactly_the_lifecycle_limit_passes_and_the_next_is_refused() {
    let limits = RateLimits {
        lifecycle: 3,
        ..wide_limits()
    };
    assert_passes_limit_then_refuses(AgentAction::Lifecycle, &limits, LimitKind::Lifecycle, 3);
}

#[test]
fn a_slot_frees_after_sixty_seconds() {
    let limits = RateLimits {
        create: 1,
        ..wide_limits()
    };
    let mut windows = RateWindows::default();
    windows
        .try_take(id(SENDER), AgentAction::Create, &limits, at_seconds(0))
        .unwrap();
    assert!(
        windows
            .try_take(id(SENDER), AgentAction::Create, &limits, at_seconds(59))
            .is_err()
    );
    assert_eq!(
        windows.try_take(id(SENDER), AgentAction::Create, &limits, at_seconds(60)),
        Ok(())
    );
}

#[test]
fn a_refused_action_takes_no_slot() {
    let limits = RateLimits {
        create: 1,
        ..wide_limits()
    };
    let mut windows = RateWindows::default();
    windows
        .try_take(id(SENDER), AgentAction::Create, &limits, at_seconds(0))
        .unwrap();
    // The refusal at 30 s must not push the free slot past 60 s.
    assert!(
        windows
            .try_take(id(SENDER), AgentAction::Create, &limits, at_seconds(30))
            .is_err()
    );
    assert_eq!(
        windows.try_take(id(SENDER), AgentAction::Create, &limits, at_seconds(60)),
        Ok(())
    );
}

#[test]
fn a_target_refusal_leaves_the_sender_window_untouched() {
    let limits = RateLimits {
        directive: 2,
        directive_target: 1,
        ..wide_limits()
    };
    let mut windows = RateWindows::default();
    let now = at_seconds(0);
    windows
        .try_take(id(SENDER), directive_to(TARGET), &limits, now)
        .unwrap();
    assert!(
        windows
            .try_take(id(SENDER), directive_to(TARGET), &limits, now)
            .is_err()
    );
    assert_eq!(
        windows.try_take(id(SENDER), directive_to(OTHER_TARGET), &limits, now),
        Ok(())
    );
}

#[test]
fn a_directive_counts_in_both_windows_and_the_target_limit_is_per_target() {
    let limits = RateLimits {
        directive: 3,
        directive_target: 2,
        ..wide_limits()
    };
    let mut windows = RateWindows::default();
    let now = at_seconds(0);
    for target in [TARGET, TARGET, OTHER_TARGET] {
        assert_eq!(
            windows.try_take(id(SENDER), directive_to(target), &limits, now),
            Ok(())
        );
    }
    // The sender window is full (3 of 3), checked before the target window.
    assert_eq!(
        windows
            .try_take(id(SENDER), directive_to(OTHER_TARGET), &limits, now)
            .unwrap_err()
            .kind,
        LimitKind::Directive
    );
}

#[test]
fn the_target_window_refuses_one_target_while_another_passes() {
    let limits = RateLimits {
        directive_target: 1,
        ..wide_limits()
    };
    let mut windows = RateWindows::default();
    let now = at_seconds(0);
    windows
        .try_take(id(SENDER), directive_to(TARGET), &limits, now)
        .unwrap();
    assert_eq!(
        windows
            .try_take(id(SENDER), directive_to(TARGET), &limits, now)
            .unwrap_err()
            .kind,
        LimitKind::DirectiveTarget
    );
    assert_eq!(
        windows.try_take(id(SENDER), directive_to(OTHER_TARGET), &limits, now),
        Ok(())
    );
}

#[test]
fn defaults_and_refusal_text_follow_the_table() {
    let limits = RateLimits::default();
    assert_eq!(
        (
            limits.directive,
            limits.directive_target,
            limits.create,
            limits.lifecycle
        ),
        (10, 3, 5, 10)
    );
    let refusal = LimitRefusal {
        kind: LimitKind::DirectiveTarget,
        allowed: 3,
    };
    assert_eq!(
        refusal.to_string(),
        "the limit \"directives sent to one target agent\" allows 3 per 60 seconds"
    );
}

#[test]
fn a_limit_of_zero_refuses_even_the_first_action() {
    let limits = RateLimits {
        lifecycle: 0,
        ..wide_limits()
    };
    let mut windows = RateWindows::default();
    assert_eq!(
        windows.try_take(id(SENDER), AgentAction::Lifecycle, &limits, at_seconds(0)),
        Err(LimitRefusal {
            kind: LimitKind::Lifecycle,
            allowed: 0
        })
    );
}

// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::json;

use super::*;

/// A swept session: one process, all results stored.
fn swept() -> SweepSummary {
    SweepSummary {
        cli_rows: 1,
        ..SweepSummary::default()
    }
}

#[test]
fn a_session_whose_processes_were_all_gone_gives_nothing() {
    assert_eq!(UncleanStop::of(&swept()), None);
}

#[test]
fn ended_processes_are_counted_when_nothing_may_run() {
    let summary = SweepSummary {
        ended: 3,
        cli_rows: 1,
        ..swept()
    };

    let finding = UncleanStop::of(&summary).expect("finding");

    assert_eq!(finding.to_json(), json!({"ended": 3}));
    assert!(!finding.warns());
}

#[test]
fn each_may_run_cause_gives_its_reason_and_the_old_warning() {
    let causes = [
        (
            SweepSummary {
                could_not_end: 1,
                ..swept()
            },
            "could not end",
        ),
        (
            SweepSummary {
                not_readable: 1,
                ..swept()
            },
            "not readable",
        ),
        (
            SweepSummary {
                boot_time_changed: 1,
                ..swept()
            },
            "boot time changed",
        ),
        (SweepSummary::default(), "no stored process"),
        (
            SweepSummary {
                helpers_capped: true,
                ..swept()
            },
            "helper limit",
        ),
        (
            SweepSummary {
                still_open: 1,
                ..swept()
            },
            "not swept",
        ),
    ];

    for (summary, reason) in causes {
        let finding = UncleanStop::of(&summary).expect("finding");
        assert_eq!(finding.to_json(), json!({ "may_run": reason }));
        assert!(finding.warns(), "{reason}");
    }
}

#[test]
fn may_run_wins_over_ended_and_the_reasons_keep_a_fixed_order() {
    let everything = SweepSummary {
        ended: 2,
        could_not_end: 1,
        not_readable: 1,
        boot_time_changed: 1,
        still_open: 1,
        cli_rows: 1,
        helpers_capped: true,
    };
    let without_could_not_end = SweepSummary {
        could_not_end: 0,
        ..everything
    };
    let without_readable = SweepSummary {
        not_readable: 0,
        ..without_could_not_end
    };
    let without_boot = SweepSummary {
        boot_time_changed: 0,
        ..without_readable
    };
    let without_cap = SweepSummary {
        helpers_capped: false,
        ..without_boot
    };

    let reasons: Vec<_> = [
        everything,
        without_could_not_end,
        without_readable,
        without_boot,
        without_cap,
    ]
    .iter()
    .map(|summary| UncleanStop::of(summary).expect("finding"))
    .collect();

    assert_eq!(
        reasons,
        [
            UncleanStop::MayRun("could not end"),
            UncleanStop::MayRun("not readable"),
            UncleanStop::MayRun("boot time changed"),
            UncleanStop::MayRun("helper limit"),
            UncleanStop::MayRun("not swept"),
        ]
    );
}

#[test]
fn helpers_without_a_stored_cli_row_still_warn() {
    let summary = SweepSummary {
        ended: 2,
        cli_rows: 0,
        ..SweepSummary::default()
    };

    assert_eq!(
        UncleanStop::of(&summary),
        Some(UncleanStop::MayRun("no stored process"))
    );
}

#[test]
fn no_stored_process_comes_before_the_helper_limit() {
    let summary = SweepSummary {
        helpers_capped: true,
        ..SweepSummary::default()
    };

    assert_eq!(
        UncleanStop::of(&summary),
        Some(UncleanStop::MayRun("no stored process"))
    );
}

// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn each_result_maps_to_the_outcome_that_resume_plan_reads() {
    let cases = [
        (LeftoverResult::Ended, SweepOutcome::Ended),
        (LeftoverResult::EndedByForce, SweepOutcome::Ended),
        (
            LeftoverResult::CouldNotEnd("denied".to_owned()),
            SweepOutcome::CouldNotEnd,
        ),
        (
            LeftoverResult::Left(LeftReason::NotReadable),
            SweepOutcome::NotReadable,
        ),
        (
            LeftoverResult::Left(LeftReason::Mismatch(Mismatch::BootTimeChanged)),
            SweepOutcome::BootTimeChanged,
        ),
        (
            LeftoverResult::Left(LeftReason::NotRunning),
            SweepOutcome::Left,
        ),
        (
            LeftoverResult::Left(LeftReason::Mismatch(Mismatch::MachineRestarted)),
            SweepOutcome::Left,
        ),
        (
            LeftoverResult::Left(LeftReason::Mismatch(Mismatch::OtherStartTime)),
            SweepOutcome::Left,
        ),
        (
            LeftoverResult::Left(LeftReason::Mismatch(Mismatch::OtherCommandLine)),
            SweepOutcome::Left,
        ),
    ];

    for (result, outcome) in cases {
        assert_eq!(result.outcome(), outcome, "{}", result.text());
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn done_report() -> FinishedReport {
    FinishedReport {
        kind: "done".to_owned(),
        text: "finished".to_owned(),
    }
}

#[test]
fn every_reason_reads_as_the_contract_writes_it() {
    let reasons = [
        (
            EndReason::Done {
                report: done_report(),
            },
            "done",
        ),
        (
            EndReason::NoOutput { minutes: 5 },
            "no output for 5 minutes",
        ),
        (EndReason::Exited { code: 1 }, "exited with code 1"),
        (
            EndReason::Failed {
                reason: "no such command".to_owned(),
            },
            "failed: no such command",
        ),
        (
            EndReason::Stopped {
                by: "the person".to_owned(),
            },
            "stopped by the person",
        ),
        (EndReason::EndedByPerson, "ended by the person"),
        (
            EndReason::ParentEnded {
                name: "Boss".to_owned(),
            },
            "its parent Boss ended",
        ),
    ];

    for (reason, text) in reasons {
        assert_eq!(reason.text(), text);
    }
}

#[test]
fn only_an_idle_end_an_exit_and_a_failure_tell_the_parent() {
    assert!(EndReason::NoOutput { minutes: 5 }.tells_parent());
    assert!(EndReason::Exited { code: 0 }.tells_parent());
    assert!(
        EndReason::Failed {
            reason: "x".to_owned()
        }
        .tells_parent()
    );
    assert!(
        !EndReason::Done {
            report: done_report()
        }
        .tells_parent()
    );
    assert!(!EndReason::EndedByPerson.tells_parent());
    assert!(
        !EndReason::Stopped {
            by: "the office".to_owned()
        }
        .tells_parent()
    );
}

#[test]
fn only_a_done_end_keeps_the_report() {
    assert_eq!(
        EndReason::Done {
            report: done_report()
        }
        .report(),
        Some(done_report())
    );
    assert_eq!(EndReason::EndedByPerson.report(), None);
}

fn status(changed_files: usize, own_commits: Option<usize>, reachable: bool) -> WorktreeStatus {
    WorktreeStatus {
        folder_exists: true,
        reachable,
        changed_files,
        status_hash: "hash".to_owned(),
        own_commits,
        base_exists: true,
        branch_commit: None,
    }
}

#[test]
fn a_worktree_holds_nothing_only_when_clean_without_commits_and_readable() {
    assert!(holds_nothing(&status(0, Some(0), true)));
    assert!(!holds_nothing(&status(1, Some(0), true)));
    assert!(!holds_nothing(&status(0, Some(2), true)));
    assert!(!holds_nothing(&status(0, None, true)));
    assert!(!holds_nothing(&status(0, Some(0), false)));
}

#[test]
fn the_parent_is_read_from_the_tree_row_of_the_temp() {
    let boss = AgentId::from_u128(1);
    let temp = AgentId::from_u128(2);
    let row = |agent_id, name: &str, parent| TreeRow {
        agent_id,
        name: name.to_owned(),
        role: None,
        parent_agent_id: parent,
        team_id: None,
    };
    let rows = [row(boss, "Boss", None), row(temp, "Temp", Some(boss))];

    assert_eq!(parent_of(&rows, temp), Some((boss, "Boss".to_owned())));
    assert_eq!(parent_of(&rows, boss), None);
    assert_eq!(parent_of(&rows, AgentId::from_u128(3)), None);
}

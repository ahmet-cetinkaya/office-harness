// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::json;

use super::*;

fn snapshot(worktree: Option<AgentWorktree>) -> TempSnapshot {
    TempSnapshot {
        agent_id: AgentId::from_u128(1),
        name: "Temp".to_owned(),
        objective: "job".to_owned(),
        creator: Actor::User,
        parent_name: None,
        started_at: Timestamp::from_unix_millis(0),
        worktree,
    }
}

fn worktree() -> AgentWorktree {
    AgentWorktree {
        repository: PathBuf::from("/repo"),
        base: "main".to_owned(),
        branch: "office-harness/temp".to_owned(),
        folder: PathBuf::from("/worktrees/temp"),
    }
}

#[test]
fn a_temp_without_a_worktree_kept_nothing() {
    let outcome = json!({"kept_folders": ["/worktrees/temp"], "kept_branches": ["x"]});

    assert_eq!(
        KeptWork::from_outcome(&snapshot(None), &outcome, true),
        KeptWork::default()
    );
}

#[test]
fn a_folder_and_branch_the_delete_kept_are_named_and_are_no_failure_when_keeping_was_the_choice() {
    let outcome = json!({
        "deleted_branches": [],
        "kept_branches": ["office-harness/temp"],
        "kept_folders": ["/worktrees/temp"],
    });

    let kept = KeptWork::from_outcome(&snapshot(Some(worktree())), &outcome, false);

    assert_eq!(kept.folder, Some(PathBuf::from("/worktrees/temp")));
    assert_eq!(kept.branch.as_deref(), Some("office-harness/temp"));
    assert_eq!(kept.removal_failure, None);
}

#[test]
fn a_folder_that_stays_after_a_remove_is_a_failure_that_names_the_worktree() {
    let outcome = json!({
        "kept_branches": ["office-harness/temp"],
        "kept_folders": ["/worktrees/temp"],
    });

    let kept = KeptWork::from_outcome(&snapshot(Some(worktree())), &outcome, true);

    assert_eq!(kept.removal_failure.as_deref(), Some(WORKTREE_STAYS));
}

#[test]
fn a_branch_that_stays_after_a_remove_is_a_failure_that_names_the_branch() {
    let outcome = json!({"kept_branches": ["office-harness/temp"], "kept_folders": []});

    let kept = KeptWork::from_outcome(&snapshot(Some(worktree())), &outcome, true);

    assert_eq!(kept.folder, None);
    assert_eq!(kept.branch.as_deref(), Some("office-harness/temp"));
    assert_eq!(kept.removal_failure.as_deref(), Some(BRANCH_STAYS));
}

#[test]
fn a_removed_worktree_and_branch_keep_nothing() {
    let outcome = json!({"deleted_branches": ["office-harness/temp"], "kept_folders": []});

    let kept = KeptWork::from_outcome(&snapshot(Some(worktree())), &outcome, true);

    assert_eq!(kept, KeptWork::default());
}

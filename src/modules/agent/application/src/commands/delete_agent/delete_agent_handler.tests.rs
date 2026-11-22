// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::enums::AgentStatus;
use tempfile::TempDir;

use super::*;
use crate::test_support::{FakeWorktrees, Office, clean_status};

#[test]
fn a_stopped_agent_is_deleted_and_an_entry_names_it() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Stopped);

    office
        .delete_agent()
        .handle(agent.id, &DeleteWorktrees::Keep)
        .expect("delete");

    assert_eq!(office.agents.all().len(), 0);
    assert_eq!(office.terminals.stopped().len(), 0);
    let entries = office.recorder.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, AgentAction::Delete);
    assert_eq!(entries[0].agent_id, agent.id);
}

#[test]
fn a_running_agent_is_stopped_before_it_is_deleted() {
    let office = Office::new();
    let folder = TempDir::new().expect("folder");
    let agent = office.add_agent("Ana", "claude", folder.path(), AgentStatus::Working);
    let other = office.add_agent("Ben", "claude", folder.path(), AgentStatus::Working);

    office
        .delete_agent()
        .handle(agent.id, &DeleteWorktrees::Keep)
        .expect("delete");

    assert_eq!(office.terminals.stopped().len(), 1);
    assert_eq!(office.terminals.stopped()[0].0, agent.id);
    assert_eq!(office.agents.all(), [other]);
}

#[test]
fn an_unknown_agent_is_not_found() {
    let office = Office::new();

    let error = office
        .delete_agent()
        .handle(AgentId::generate(), &DeleteWorktrees::Keep)
        .expect_err("unknown");

    assert!(matches!(error, ApplicationError::NotFound(_)));
    assert_eq!(office.recorder.entries().len(), 0);
}

/// An agent with the fake's worktree, stored as the create would store it.
fn agent_with_worktree(office: &Office, worktrees: &FakeWorktrees, status: AgentStatus) -> Agent {
    let mut agent = office.add_agent("Ana", "claude", &worktrees.folder, status);
    office.agents.delete(agent.id).expect("delete");
    agent.worktree = Some(worktrees.worktree());
    office.agents.insert(&agent).expect("insert");
    agent
}

fn remove_confirming(agent: &Agent, hash: &str) -> DeleteWorktrees {
    DeleteWorktrees::Remove(HashMap::from([(agent.id, hash.to_owned())]))
}

#[test]
fn keep_leaves_the_folder_and_the_branch_and_names_them() {
    let office = Office::new();
    let worktrees = FakeWorktrees::new(&std::env::temp_dir().join("wt"));
    let agent = agent_with_worktree(&office, &worktrees, AgentStatus::Stopped);

    let outcome = office
        .delete_agent_with(&worktrees)
        .handle(agent.id, &DeleteWorktrees::Keep)
        .expect("delete");

    assert_eq!(outcome.kept_branches, ["office-harness/ana"]);
    assert_eq!(
        outcome.kept_folders,
        [worktrees.folder.to_string_lossy().into_owned()]
    );
    assert_eq!(outcome.deleted_branches.len(), 0);
    assert_eq!(worktrees.calls().len(), 0);
    assert_eq!(office.agents.all().len(), 0);
}

#[test]
fn a_clean_confirmed_worktree_is_removed_without_force_with_its_branch() {
    let office = Office::new();
    let worktrees = FakeWorktrees::new(&std::env::temp_dir().join("wt"));
    let agent = agent_with_worktree(&office, &worktrees, AgentStatus::Stopped);

    let outcome = office
        .delete_agent_with(&worktrees)
        .handle(agent.id, &remove_confirming(&agent, "h0"))
        .expect("delete");

    assert_eq!(outcome.deleted_branches, ["office-harness/ana"]);
    assert_eq!(outcome.kept_folders.len(), 0);
    // A stopped agent writes nothing, so one status read is enough.
    assert_eq!(worktrees.calls(), ["status", "remove", "branch -D"]);
    assert_eq!(office.agents.all().len(), 0);
}

#[test]
fn a_record_that_fails_after_the_removal_names_what_went() {
    let office = Office::new();
    let worktrees = FakeWorktrees::new(&std::env::temp_dir().join("wt"));
    let agent = agent_with_worktree(&office, &worktrees, AgentStatus::Stopped);
    office.recorder.fail_from_now();

    let error = office
        .delete_agent_with(&worktrees)
        .handle(agent.id, &remove_confirming(&agent, "h0"))
        .expect_err("not recorded");

    let ApplicationError::Storage { message, .. } = &error else {
        panic!("{error:?}");
    };
    assert!(
        message.contains("The agent was deleted; deleted branches: office-harness/ana"),
        "{message}"
    );
    assert_eq!(office.agents.all().len(), 0);
}

#[test]
fn a_worktree_with_changes_is_removed_with_force_only_when_confirmed() {
    let office = Office::new();
    let worktrees = FakeWorktrees::new(&std::env::temp_dir().join("wt"));
    let agent = agent_with_worktree(&office, &worktrees, AgentStatus::Stopped);
    worktrees.queue_statuses(vec![WorktreeStatus {
        changed_files: 2,
        ..clean_status("h2")
    }]);

    let error = office
        .delete_agent_with(&worktrees)
        .handle(agent.id, &DeleteWorktrees::Remove(HashMap::new()))
        .expect_err("not confirmed");
    assert_eq!(
        error,
        ApplicationError::Conflict("the worktree of Ana changed; confirm again".to_owned())
    );
    assert_eq!(office.agents.all().len(), 1);

    office
        .delete_agent_with(&worktrees)
        .handle(agent.id, &remove_confirming(&agent, "h2"))
        .expect("delete");
    assert!(worktrees.calls().contains(&"remove --force".to_owned()));
}

#[test]
fn a_change_between_the_question_and_the_stop_keeps_everything() {
    let office = Office::new();
    let worktrees = FakeWorktrees::new(&std::env::temp_dir().join("wt"));
    let agent = agent_with_worktree(&office, &worktrees, AgentStatus::Working);
    // The running agent writes a file before it stops.
    worktrees.queue_statuses(vec![clean_status("h0"), clean_status("h1")]);

    let error = office
        .delete_agent_with(&worktrees)
        .handle(agent.id, &remove_confirming(&agent, "h0"))
        .expect_err("changed");

    assert!(matches!(error, ApplicationError::Conflict(_)));
    assert_eq!(office.terminals.stopped().len(), 1);
    assert_eq!(worktrees.calls(), ["status", "status"]);
    assert_eq!(office.agents.all().len(), 1);
}

#[test]
fn a_branch_with_own_commits_or_without_its_base_is_kept() {
    for status in [
        WorktreeStatus {
            own_commits: Some(3),
            ..clean_status("h0")
        },
        WorktreeStatus {
            base_exists: false,
            own_commits: None,
            ..clean_status("h0")
        },
    ] {
        let office = Office::new();
        let worktrees = FakeWorktrees::new(&std::env::temp_dir().join("wt"));
        let agent = agent_with_worktree(&office, &worktrees, AgentStatus::Stopped);
        worktrees.queue_statuses(vec![status]);

        let outcome = office
            .delete_agent_with(&worktrees)
            .handle(agent.id, &remove_confirming(&agent, "h0"))
            .expect("delete");

        assert_eq!(outcome.kept_branches, ["office-harness/ana"]);
        assert_eq!(outcome.kept_folders.len(), 0);
        assert!(!worktrees.calls().contains(&"branch -D".to_owned()));
    }
}

#[test]
fn an_unreachable_repository_keeps_the_worktree_and_still_deletes_the_agent() {
    let office = Office::new();
    let worktrees = FakeWorktrees::new(&std::env::temp_dir().join("wt"));
    let agent = agent_with_worktree(&office, &worktrees, AgentStatus::Stopped);
    worktrees.queue_statuses(vec![WorktreeStatus {
        reachable: false,
        ..clean_status("")
    }]);

    let outcome = office
        .delete_agent_with(&worktrees)
        .handle(agent.id, &DeleteWorktrees::Remove(HashMap::new()))
        .expect("delete");

    assert_eq!(outcome.kept_branches, ["office-harness/ana"]);
    assert_eq!(outcome.kept_folders.len(), 1);
    assert_eq!(office.agents.all().len(), 0);
}

#[test]
fn a_remove_git_refuses_keeps_the_worktree() {
    let office = Office::new();
    let worktrees = FakeWorktrees::new(&std::env::temp_dir().join("wt"));
    let agent = agent_with_worktree(&office, &worktrees, AgentStatus::Stopped);
    *worktrees.remove_fails.lock().expect("lock") = true;

    let outcome = office
        .delete_agent_with(&worktrees)
        .handle(agent.id, &remove_confirming(&agent, "h0"))
        .expect("delete");

    assert_eq!(outcome.kept_folders.len(), 1);
    assert_eq!(outcome.kept_branches, ["office-harness/ana"]);
}

#[test]
fn discard_removes_without_asking() {
    let office = Office::new();
    let worktrees = FakeWorktrees::new(&std::env::temp_dir().join("wt"));
    let agent = agent_with_worktree(&office, &worktrees, AgentStatus::Stopped);

    let outcome = office
        .delete_agent_with(&worktrees)
        .handle(agent.id, &DeleteWorktrees::Discard)
        .expect("delete");

    assert_eq!(outcome, WorktreeOutcome::default());
    assert_eq!(worktrees.calls(), ["discard"]);
    assert_eq!(office.agents.all().len(), 0);
}

#[test]
fn the_check_refuses_a_stale_hash_and_changes_nothing() {
    let office = Office::new();
    let worktrees = FakeWorktrees::new(&std::env::temp_dir().join("wt"));
    let agent = agent_with_worktree(&office, &worktrees, AgentStatus::Working);
    let handler = office.delete_agent_with(&worktrees);

    assert!(
        handler
            .check(agent.id, &remove_confirming(&agent, "h0"))
            .is_ok()
    );
    assert!(matches!(
        handler.check(agent.id, &remove_confirming(&agent, "old")),
        Err(ApplicationError::Conflict(_))
    ));
    assert!(handler.check(agent.id, &DeleteWorktrees::Keep).is_ok());
    assert_eq!(office.terminals.stopped().len(), 0);
    assert_eq!(office.agents.all().len(), 1);
}

#[test]
fn stop_and_check_stops_and_names_the_stop_in_its_refusal() {
    let office = Office::new();
    let worktrees = FakeWorktrees::new(&std::env::temp_dir().join("wt"));
    let agent = agent_with_worktree(&office, &worktrees, AgentStatus::Working);
    worktrees.queue_statuses(vec![clean_status("h1")]);

    let error = office
        .delete_agent_with(&worktrees)
        .stop_and_check(agent.id, &remove_confirming(&agent, "h0"))
        .expect_err("changed");

    assert_eq!(
        error,
        ApplicationError::Conflict(
            "the worktree of Ana changed while it stopped; it is stopped now, confirm again"
                .to_owned()
        )
    );
    assert_eq!(office.terminals.stopped().len(), 1);
    assert_eq!(office.agents.all().len(), 1);
}

#[test]
fn stop_and_check_reads_nothing_for_an_agent_that_was_already_stopped() {
    let office = Office::new();
    let worktrees = FakeWorktrees::new(&std::env::temp_dir().join("wt"));
    let agent = agent_with_worktree(&office, &worktrees, AgentStatus::Stopped);

    office
        .delete_agent_with(&worktrees)
        .stop_and_check(agent.id, &remove_confirming(&agent, "h0"))
        .expect("checked");

    assert_eq!(worktrees.calls().len(), 0);
    assert_eq!(office.terminals.stopped().len(), 0);
}

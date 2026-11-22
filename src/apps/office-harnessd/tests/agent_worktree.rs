// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(unix)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! Own worktrees per agent (0010) through a real server and real git
//! repositories: the info and status requests, create, start, keep or remove
//! on delete, one question for a subtree, and the action record.

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use agent_infrastructure::services::find_program;
use serde_json::{Value, json};
use support::fake_cli::FakeClis;
use support::office_client::{Client, Office, error_text};
use tempfile::TempDir;

/// A CLI that keeps running until it is stopped.
const WAITING_CLI: &str = "echo ready\nwhile true; do sleep 0.1; done";
const SC_002_AGENTS: usize = 10;

/// An office whose login `PATH` finds the real git, and a repository with
/// `a.txt` and `sub/b.txt` on `main`.
struct Fixture {
    office: Office,
    client: Client,
    repository_folder: TempDir,
}

impl Fixture {
    async fn open() -> Self {
        let clis = FakeClis::new();
        let environment = std::env::vars().collect();
        let git_program =
            find_program("git", &environment, Path::new("/")).expect("git is installed");
        clis.add("git", &format!("exec '{}' \"$@\"", git_program.display()));
        clis.add("waiting-cli", WAITING_CLI);
        let office = Office::open_with(clis).await;
        let client = office.client().await;
        let repository_folder = tempfile::tempdir().expect("repository folder");
        let fixture = Self {
            office,
            client,
            repository_folder,
        };
        let repository = fixture.repository();
        fs::create_dir_all(repository.join("sub")).expect("folders");
        git(&repository, &["init", "--quiet", "--initial-branch=main"]);
        fs::write(repository.join("a.txt"), "a\n").expect("file");
        fs::write(repository.join("sub").join("b.txt"), "b\n").expect("file");
        commit_all(&repository, "first");
        fixture
    }

    fn repository(&self) -> PathBuf {
        self.repository_folder.path().join("repo")
    }

    /// Creates an agent with its own worktree and returns its record.
    async fn create_with_worktree(&mut self, name: &str) -> Value {
        let answer = self
            .client
            .ask(
                "agent.create",
                json!({
                    "name": name,
                    "command": "waiting-cli",
                    "working_folder": self.repository(),
                    "own_worktree": true,
                }),
            )
            .await;
        assert_eq!(answer["ok"], true, "{answer}");
        answer["result"].clone()
    }

    async fn status_of(&mut self, agent_ids: &[&str]) -> Vec<Value> {
        let answer = self
            .client
            .ask("agent.worktree_status", json!({"agent_ids": agent_ids}))
            .await;
        assert_eq!(answer["ok"], true, "{answer}");
        answer["result"].as_array().expect("list").clone()
    }

    /// Deletes with `remove`, confirming the hashes of `statuses`.
    async fn remove(&mut self, agent_id: &str, statuses: &[Value], extra: Value) -> Value {
        let confirmed: serde_json::Map<String, Value> = statuses
            .iter()
            .map(|status| {
                (
                    status["agent_id"].as_str().expect("id").to_owned(),
                    status["status_hash"].clone(),
                )
            })
            .collect();
        let mut fields = json!({
            "agent_id": agent_id,
            "worktrees": "remove",
            "confirmed_status": confirmed,
        });
        if let (Value::Object(fields), Value::Object(extra)) = (&mut fields, extra) {
            fields.extend(extra);
        }
        self.client.ask("agent.delete", fields).await
    }

    fn branches(&self) -> String {
        git(&self.repository(), &["branch", "--list"])
    }

    fn worktree_count(&self) -> usize {
        git(&self.repository(), &["worktree", "list", "--porcelain"])
            .matches("worktree ")
            .count()
    }
}

fn id_of(agent: &Value) -> String {
    agent["id"].as_str().expect("id").to_owned()
}

fn folder_of(agent: &Value) -> PathBuf {
    PathBuf::from(agent["worktree"]["folder"].as_str().expect("folder"))
}

/// The real git with no user configuration, for the fixture's own commits.
fn git(folder: &Path, arguments: &[&str]) -> String {
    let home = folder.parent().expect("parent").join("home");
    fs::create_dir_all(&home).expect("home");
    let output = Command::new("git")
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "-C",
        ])
        .arg(folder)
        .args(arguments)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", &home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {arguments:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn commit_all(folder: &Path, message: &str) {
    git(folder, &["add", "--all"]);
    git(folder, &["commit", "--quiet", "-m", message]);
}

#[tokio::test]
async fn the_choice_is_offered_only_inside_a_repository() {
    let mut fixture = Fixture::open().await;
    let outside = fixture.office.work_path().to_owned();

    let answer = fixture
        .client
        .ask(
            "agent.worktree_info",
            json!({"folder": outside, "name": "Ana"}),
        )
        .await;
    assert_eq!(
        answer["result"],
        json!({"in_repository": false}),
        "{answer}"
    );

    let answer = fixture
        .client
        .ask(
            "agent.worktree_info",
            json!({"folder": fixture.repository().join("sub"), "name": "Lead Dev"}),
        )
        .await;
    assert_eq!(
        answer["result"],
        json!({
            "in_repository": true,
            "base": "main",
            "base_is_branch": true,
            "branches": ["main"],
            "branch_name": "office-harness/lead-dev",
            "subfolder": "sub/",
        }),
        "{answer}"
    );

    // With the choice left at its default no worktree is made (SC-004).
    let plain = fixture
        .client
        .create_agent(&fixture.office, "Plain", "waiting-cli", None)
        .await;
    let listed = fixture.client.ask("agent.list", json!({})).await;
    let record = &listed["result"][0];
    assert_eq!(record["id"], json!(plain));
    assert_eq!(record["worktree"], Value::Null);
    assert_eq!(fixture.worktree_count(), 1);
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn outside_a_repository_the_create_is_refused_and_makes_nothing() {
    let mut fixture = Fixture::open().await;
    let answer = fixture
        .client
        .ask(
            "agent.create",
            json!({
                "name": "Ana",
                "command": "waiting-cli",
                "working_folder": fixture.office.work_path(),
                "own_worktree": true,
            }),
        )
        .await;
    assert_eq!(
        error_text(&answer),
        ("invalid", "own worktree needs a git repository")
    );
    let listed = fixture.client.ask("agent.list", json!({})).await;
    assert_eq!(listed["result"], json!([]));
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn two_agents_change_the_same_file_only_in_their_own_worktrees() {
    let mut fixture = Fixture::open().await;
    let first = fixture.create_with_worktree("Ana").await;
    let second = fixture.create_with_worktree("Ana 2").await;
    assert_eq!(first["worktree"]["branch"], "office-harness/ana");
    assert_eq!(first["worktree"]["base"], "main");
    assert_eq!(second["worktree"]["branch"], "office-harness/ana-2");
    assert_eq!(first["working_folder"], first["worktree"]["folder"]);

    fs::write(folder_of(&first).join("a.txt"), "from Ana\n").expect("write");
    fs::write(folder_of(&second).join("a.txt"), "from Ana 2\n").expect("write");

    let read = |agent: &Value| fs::read_to_string(folder_of(agent).join("a.txt")).expect("read");
    assert_eq!(read(&first), "from Ana\n");
    assert_eq!(read(&second), "from Ana 2\n");
    assert_eq!(git(&fixture.repository(), &["status", "--porcelain"]), "");

    // The CLI starts in the worktree, and its status shows the one change.
    fixture.client.start(&id_of(&first)).await;
    fixture
        .client
        .wait_for_status(&id_of(&first), "waiting_for_input")
        .await;
    let statuses = fixture.status_of(&[&id_of(&first)]).await;
    assert_eq!(statuses[0]["changed_files"], 1);
    assert_eq!(statuses[0]["own_commits"], 0);
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn ten_clean_worktrees_removed_leave_no_folder_record_or_branch() {
    let mut fixture = Fixture::open().await;
    let mut agents = Vec::new();
    for number in 0..SC_002_AGENTS {
        agents.push(fixture.create_with_worktree(&format!("Dev {number}")).await);
    }
    assert_eq!(fixture.worktree_count(), SC_002_AGENTS + 1);

    for agent in &agents {
        let statuses = fixture.status_of(&[&id_of(agent)]).await;
        let answer = fixture.remove(&id_of(agent), &statuses, json!({})).await;
        assert_eq!(answer["ok"], true, "{answer}");
        assert_eq!(answer["result"]["kept_folders"], json!([]), "{answer}");
        assert_eq!(
            answer["result"]["deleted_branches"]
                .as_array()
                .map(Vec::len),
            Some(1)
        );
    }

    assert_eq!(fixture.worktree_count(), 1);
    assert!(
        !fixture.branches().contains("office-harness/"),
        "{}",
        fixture.branches()
    );
    assert!(agents.iter().all(|agent| !folder_of(agent).exists()));
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn changes_are_removed_only_with_the_hash_the_user_confirmed() {
    let mut fixture = Fixture::open().await;
    let agent = fixture.create_with_worktree("Ana").await;
    let folder = folder_of(&agent);
    fs::write(folder.join("a.txt"), "changed\n").expect("write");

    let unconfirmed = fixture
        .client
        .ask(
            "agent.delete",
            json!({"agent_id": id_of(&agent), "worktrees": "remove"}),
        )
        .await;
    assert_eq!(
        error_text(&unconfirmed),
        ("conflict", "the worktree of Ana changed; confirm again"),
        "{unconfirmed}"
    );

    // One file reverted and another added: the same count, a new hash.
    let statuses = fixture.status_of(&[&id_of(&agent)]).await;
    assert_eq!(statuses[0]["changed_files"], 1);
    fs::write(folder.join("a.txt"), "a\n").expect("revert");
    fs::write(folder.join("c.txt"), "c\n").expect("add");
    let stale = fixture.remove(&id_of(&agent), &statuses, json!({})).await;
    assert_eq!(error_text(&stale).0, "conflict", "{stale}");
    assert!(folder.join("c.txt").is_file());

    let statuses = fixture.status_of(&[&id_of(&agent)]).await;
    let removed = fixture.remove(&id_of(&agent), &statuses, json!({})).await;
    assert_eq!(removed["ok"], true, "{removed}");
    assert!(!folder.exists());
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn a_file_the_running_agent_writes_at_its_stop_refuses_the_removal() {
    let mut fixture = Fixture::open().await;
    fixture.office.clis.add(
        "writing-cli",
        "trap 'echo late > late.txt; exit 0' INT TERM HUP\necho ready\nwhile true; do sleep 0.1; done",
    );
    let agent = fixture
        .client
        .ask(
            "agent.create",
            json!({
                "name": "Ana",
                "command": "writing-cli",
                "working_folder": fixture.repository(),
                "own_worktree": true,
            }),
        )
        .await["result"]
        .clone();
    fixture.client.start(&id_of(&agent)).await;
    fixture
        .client
        .wait_for_status(&id_of(&agent), "waiting_for_input")
        .await;
    let statuses = fixture.status_of(&[&id_of(&agent)]).await;
    assert_eq!(statuses[0]["changed_files"], 0);

    let answer = fixture.remove(&id_of(&agent), &statuses, json!({})).await;

    assert_eq!(error_text(&answer).0, "conflict", "{answer}");
    assert!(folder_of(&agent).join("late.txt").is_file());
    assert!(fixture.branches().contains("office-harness/ana"));
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn keep_and_own_commits_and_a_gone_repository_keep_and_name_what_stays() {
    let mut fixture = Fixture::open().await;
    let kept = fixture.create_with_worktree("Kept").await;
    let answer = fixture
        .client
        .ask("agent.delete", json!({"agent_id": id_of(&kept)}))
        .await;
    assert_eq!(
        answer["result"]["kept_branches"],
        json!(["office-harness/kept"])
    );
    assert_eq!(
        answer["result"]["kept_folders"],
        json!([folder_of(&kept).to_string_lossy()])
    );
    assert!(folder_of(&kept).join("a.txt").is_file());

    let committed = fixture.create_with_worktree("Committed").await;
    fs::write(folder_of(&committed).join("new.txt"), "x\n").expect("write");
    commit_all(&folder_of(&committed), "own work");
    let statuses = fixture.status_of(&[&id_of(&committed)]).await;
    assert_eq!(statuses[0]["own_commits"], 1);
    let answer = fixture
        .remove(&id_of(&committed), &statuses, json!({}))
        .await;
    assert_eq!(
        answer["result"]["kept_branches"],
        json!(["office-harness/committed"]),
        "{answer}"
    );
    assert_eq!(answer["result"]["kept_folders"], json!([]));
    assert!(fixture.branches().contains("office-harness/committed"));

    let orphan = fixture.create_with_worktree("Orphan").await;
    let moved = fixture.repository_folder.path().join("moved");
    fs::rename(fixture.repository(), &moved).expect("move the repository");
    let statuses = fixture.status_of(&[&id_of(&orphan)]).await;
    assert_eq!(statuses[0]["reachable"], false);
    let answer = fixture.remove(&id_of(&orphan), &statuses, json!({})).await;
    assert_eq!(answer["ok"], true, "{answer}");
    assert_eq!(
        answer["result"]["kept_folders"],
        json!([folder_of(&orphan).to_string_lossy()])
    );
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn a_start_after_the_worktree_folder_is_gone_is_refused() {
    let mut fixture = Fixture::open().await;
    let agent = fixture.create_with_worktree("Ana").await;
    fs::remove_dir_all(folder_of(&agent)).expect("deleted by hand");

    let answer = fixture
        .client
        .ask("agent.start", json!({"agent_id": id_of(&agent)}))
        .await;

    assert_eq!(
        error_text(&answer),
        (
            "invalid",
            &*format!(
                "the worktree folder is missing or broken: {}",
                folder_of(&agent).display()
            )
        )
    );
    // Delete still offers remove, which clears the repository's record.
    let statuses = fixture.status_of(&[&id_of(&agent)]).await;
    assert_eq!(statuses[0]["folder_exists"], false);
    let removed = fixture.remove(&id_of(&agent), &statuses, json!({})).await;
    assert_eq!(removed["ok"], true, "{removed}");
    assert_eq!(fixture.worktree_count(), 1);
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn a_continue_after_the_worktree_folder_is_gone_is_refused_and_starts_nothing() {
    let mut fixture = Fixture::open().await;
    fixture.office.clis.add("claude", "echo ready");
    let created = fixture
        .client
        .ask(
            "agent.create",
            json!({
                "name": "Ana",
                "command": "claude",
                "working_folder": fixture.repository(),
                "own_worktree": true,
            }),
        )
        .await;
    assert_eq!(created["ok"], true, "{created}");
    let agent = created["result"].clone();
    let first = fixture.client.start(&id_of(&agent)).await;
    fixture
        .client
        .wait_for_status(&id_of(&agent), "exited")
        .await;
    fs::remove_dir_all(folder_of(&agent)).expect("deleted by hand");

    let answer = fixture
        .client
        .ask("session.continue", json!({"session_id": first["id"]}))
        .await;

    assert_eq!(
        error_text(&answer),
        (
            "invalid",
            &*format!(
                "the worktree folder is missing or broken: {}",
                folder_of(&agent).display()
            )
        )
    );
    assert_eq!(fixture.client.sessions(&id_of(&agent)).await.len(), 1);
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn a_subtree_delete_asks_once_and_a_stale_member_stops_it_before_any_delete() {
    let mut fixture = Fixture::open().await;
    let chief = fixture
        .client
        .ask(
            "agent.create",
            json!({
                "name": "Chief",
                "command": "waiting-cli",
                "working_folder": fixture.office.work_path(),
                "role": {"kind": "ceo"},
            }),
        )
        .await;
    assert_eq!(chief["ok"], true, "{chief}");
    let lead = fixture.create_with_worktree("Lead").await;
    let team = fixture
        .client
        .ask(
            "team.create",
            json!({"name": "The Team", "lead_agent_id": id_of(&lead)}),
        )
        .await;
    assert_eq!(team["ok"], true, "{team}");
    let mut members = Vec::new();
    for name in ["One", "Two", "Three"] {
        // The role create carries the choice too (RoleSeed).
        let member = fixture
            .client
            .ask(
                "agent.create",
                json!({
                    "name": name,
                    "command": "waiting-cli",
                    "working_folder": fixture.repository(),
                    "own_worktree": true,
                    "role": {"kind": "member", "parent_agent_id": id_of(&lead)},
                }),
            )
            .await;
        assert_eq!(member["ok"], true, "{member}");
        members.push(member["result"].clone());
    }
    let ids: Vec<String> = std::iter::once(&lead).chain(&members).map(id_of).collect();
    let id_refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let statuses = fixture.status_of(&id_refs).await;
    assert_eq!(statuses.len(), 4);
    let subtree = json!({"include_descendants": true, "descendant_count": 3});

    fs::write(folder_of(&members[2]).join("late.txt"), "x").expect("write");
    let stale = fixture
        .remove(&id_of(&lead), &statuses, subtree.clone())
        .await;
    assert_eq!(error_text(&stale).0, "conflict", "{stale}");
    assert_eq!(fixture.worktree_count(), 5);

    let statuses = fixture.status_of(&id_refs).await;
    let answer = fixture.remove(&id_of(&lead), &statuses, subtree).await;
    assert_eq!(answer["ok"], true, "{answer}");
    assert_eq!(
        answer["result"]["deleted_branches"]
            .as_array()
            .map(Vec::len),
        Some(4)
    );
    assert_eq!(fixture.worktree_count(), 1);
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn each_create_and_delete_is_recorded_with_its_actor() {
    let mut fixture = Fixture::open().await;
    let agent = fixture.create_with_worktree("Ana").await;
    let statuses = fixture.status_of(&[&id_of(&agent)]).await;
    fixture.remove(&id_of(&agent), &statuses, json!({})).await;

    let actions = fixture
        .client
        .ask("agent.actions", json!({"agent_id": id_of(&agent)}))
        .await;
    let entries = actions["result"].as_array().expect("entries");
    let kinds: Vec<(&str, &str)> = entries
        .iter()
        .map(|entry| {
            (
                entry["action"].as_str().unwrap_or_default(),
                entry["actor"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert!(kinds.contains(&("create", "user")), "{actions}");
    assert!(kinds.contains(&("delete", "user")), "{actions}");
    fixture.office.serving.stop().await;
}

#[tokio::test]
async fn a_refused_role_create_with_an_own_worktree_leaves_nothing() {
    let mut fixture = Fixture::open().await;
    let answer = fixture
        .client
        .ask(
            "agent.create",
            json!({
                "name": "Orphan",
                "command": "waiting-cli",
                "working_folder": fixture.repository(),
                "own_worktree": true,
                "role": {"kind": "member", "parent_agent_id": "00000000-0000-4000-8000-000000000000"},
            }),
        )
        .await;
    assert_eq!(answer["ok"], false, "{answer}");
    let listed = fixture.client.ask("agent.list", json!({})).await;
    assert_eq!(listed["result"], json!([]));
    assert!(
        !fixture.branches().contains("office-harness/"),
        "{}",
        fixture.branches()
    );
    assert_eq!(fixture.worktree_count(), 1);
    fixture.office.serving.stop().await;
}

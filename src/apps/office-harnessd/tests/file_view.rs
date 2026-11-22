// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(unix)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! The read-only file browser of an agent (0016) through a real server, a
//! user client, and real git repositories: each request, the refusals, the
//! secret-looking files, the agent connection, the action record, and the
//! timings.

mod support;

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use agent_infrastructure::services::find_program;
use serde_json::{Value, json};
use support::fake_cli::FakeClis;
use support::office_client::{Client, Office, error_text, wait_for};
use tempfile::TempDir;

/// The real server binary that the fake CLI runs as the helper.
const HELPER: &str = env!("CARGO_BIN_EXE_office-harnessd");
/// A well-formed agent id that no agent has.
const UNKNOWN_AGENT: &str = "00000000-0000-4000-8000-000000000000";
const OUTSIDE_MARKER: &str = "OUTSIDE-SECRET-BYTES";
const SECRET_MARKER: &str = "SECRETVALUE";
const FILES_IN_FOLDER: usize = 50_000;
const ENTRIES_PER_FOLDER: usize = 1_000;
const LIST_LIMIT: Duration = Duration::from_secs(2);
const READ_LIMIT: Duration = Duration::from_secs(2);
const CHANGES_LIMIT: Duration = Duration::from_secs(5);
const MAX_CONTENT_BYTES: usize = 524_288;
const BASE64_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// An office whose login `PATH` finds the real git, an agent whose working
/// folder is a repository with `a.txt` and `sub/b.txt`, and a folder
/// outside it that holds a file with a recognizable content.
struct Fixture {
    office: Office,
    client: Client,
    agent: String,
    folders: TempDir,
}

impl Fixture {
    async fn open() -> Self {
        let clis = FakeClis::new();
        let environment = std::env::vars().collect();
        let git_program =
            find_program("git", &environment, Path::new("/")).expect("git is installed");
        clis.add("git", &format!("exec '{}' \"$@\"", git_program.display()));
        clis.add("claude", "sleep 60");
        let office = Office::open_with(clis).await;
        let mut client = office.client().await;
        let folders = tempfile::tempdir().expect("folders");
        let repository = folders.path().join("repo");
        fs::create_dir_all(repository.join("sub")).expect("repository");
        let outside = folders.path().join("outside");
        fs::create_dir_all(outside.join("inner")).expect("outside folder");
        fs::write(outside.join("key.txt"), OUTSIDE_MARKER).expect("outside file");
        fs::write(outside.join("inner/deep.txt"), OUTSIDE_MARKER).expect("outside file");
        git(&repository, &["init", "--quiet", "--initial-branch=main"]);
        fs::write(repository.join("a.txt"), "one\n").expect("file");
        fs::write(repository.join("sub/b.txt"), "b\n").expect("file");
        commit_all(&repository, "first");
        let created = client
            .ask(
                "agent.create",
                json!({"name": "Ana", "command": "claude", "working_folder": repository}),
            )
            .await;
        assert_eq!(created["ok"], true, "{created}");
        let agent = created["result"]["id"].as_str().expect("id").to_owned();
        Self {
            office,
            client,
            agent,
            folders,
        }
    }

    fn repository(&self) -> PathBuf {
        self.folders.path().join("repo")
    }

    fn outside(&self) -> PathBuf {
        self.folders.path().join("outside")
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.repository().join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("folders");
        fs::write(path, content).expect("file");
    }

    /// Asks a files request for this fixture's agent.
    async fn files(&mut self, kind: &str, fields: Value) -> Value {
        let mut body = fields;
        body["agent_id"] = json!(self.agent);
        self.client.ask(&format!("agent.files.{kind}"), body).await
    }

    async fn ok(&mut self, kind: &str, fields: Value) -> Value {
        let answer = self.files(kind, fields).await;
        assert_eq!(answer["ok"], true, "{answer}");
        answer["result"].clone()
    }

    async fn failure(&mut self, kind: &str, fields: Value) -> String {
        let answer = self.files(kind, fields).await;
        assert_eq!(answer["ok"], false, "{answer}");
        error_text(&answer).0.to_owned()
    }

    fn head(&self) -> String {
        git(&self.repository(), &["rev-parse", "HEAD"])
            .trim()
            .to_owned()
    }
}

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

/// The bytes of a `content_base64` field.
fn decoded(content_base64: &Value) -> Vec<u8> {
    let text = content_base64.as_str().expect("base64 text");
    let mut bytes = Vec::new();
    let mut packed = 0_u32;
    let mut bits = 0;
    for character in text.bytes().take_while(|character| *character != b'=') {
        let value = BASE64_ALPHABET
            .iter()
            .position(|candidate| *candidate == character)
            .expect("a base64 character");
        packed = (packed << 6) | u32::try_from(value).expect("small");
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            bytes.push(u8::try_from((packed >> bits) & 0xff).expect("a byte"));
        }
    }
    bytes
}

fn text_of(result: &Value) -> String {
    String::from_utf8(decoded(&result["content_base64"])).expect("utf-8")
}

fn names_of(result: &Value, list: &str) -> Vec<String> {
    result[list]
        .as_array()
        .expect("list")
        .iter()
        .map(|item| {
            item["name"]
                .as_str()
                .or_else(|| item["path"].as_str())
                .expect("a name or path")
                .to_owned()
        })
        .collect()
}

#[tokio::test]
async fn every_request_answers_with_its_contract_fields() {
    let mut fixture = Fixture::open().await;
    fixture.write("sub/b.txt", "b2\n");
    git(
        &fixture.repository(),
        &["commit", "--quiet", "-am", "second"],
    );
    fixture.write("a.txt", "two\n");
    fixture.write("new.txt", "fresh\n");
    let repository = fs::canonicalize(fixture.repository()).expect("canonical");

    let root = fixture.ok("list", json!({"path": ""})).await;
    let sub = fixture.ok("list", json!({"path": "sub"})).await;
    let file = fixture.ok("read", json!({"path": "a.txt"})).await;
    let changes = fixture.ok("changes", json!({})).await;
    let diff = fixture.ok("diff", json!({"path": "a.txt"})).await;
    let history = fixture.ok("history", json!({})).await;
    let commit = fixture
        .ok("commit", json!({"commit": fixture.head()}))
        .await;

    assert_eq!(root["root"], repository.to_string_lossy().as_ref());
    assert_eq!(root["path"], "");
    assert_eq!(names_of(&root, "entries"), ["sub", "a.txt", "new.txt"]);
    assert_eq!(
        root["entries"][1],
        json!({"name": "a.txt", "kind": "file", "size": 4, "secret": false, "target": null})
    );
    assert_eq!(
        (root["more"].clone(), root["read_at"].is_number()),
        (json!(0), true)
    );
    assert_eq!(names_of(&sub, "entries"), ["b.txt"]);
    assert_eq!(file["path"], "a.txt");
    assert_eq!(
        (file["state"].clone(), file["size"].clone()),
        (json!("text"), json!(4))
    );
    assert_eq!(text_of(&file), "two\n");
    assert_eq!(changes["repository"], true);
    assert_eq!(
        changes["files"],
        json!([
            {"path": "a.txt", "kind": "modified", "old_path": null, "secret": false},
            {"path": "new.txt", "kind": "untracked", "old_path": null, "secret": false},
        ])
    );
    assert_eq!(diff["state"], "diff");
    assert!(text_of(&diff).starts_with("@@ "), "{diff}");
    assert!(text_of(&diff).contains("-one\n+two\n"));
    assert_eq!(history["repository"], true);
    assert_eq!(history["more"], false);
    let subjects: Vec<&str> = history["commits"]
        .as_array()
        .expect("commits")
        .iter()
        .map(|commit| commit["subject"].as_str().expect("subject"))
        .collect();
    assert_eq!(subjects, ["second", "first"]);
    assert_eq!(history["commits"][0]["id"], fixture.head());
    assert_eq!(commit["id"], fixture.head());
    assert_eq!(commit["message"], "second");
    assert_eq!(commit["author"], "Test");
    assert_eq!(commit["email"], "test@example.com");
    assert_eq!(commit["parents"].as_array().expect("parents").len(), 1);
    assert_eq!(names_of(&commit, "files"), ["sub/b.txt"]);
}

#[tokio::test]
async fn bad_requests_get_the_codes_of_the_contract() {
    let mut fixture = Fixture::open().await;
    let tree = git(&fixture.repository(), &["rev-parse", "HEAD^{tree}"])
        .trim()
        .to_owned();

    let answer = fixture
        .client
        .ask(
            "agent.files.list",
            json!({"agent_id": UNKNOWN_AGENT, "path": ""}),
        )
        .await;
    assert_eq!(error_text(&answer).0, "not_found", "{answer}");

    for path in [
        "/etc/passwd",
        "a/../b",
        "a//b",
        "./a",
        "a/",
        "a\u{0}b",
        "..",
    ] {
        for kind in ["list", "read"] {
            let code = fixture.failure(kind, json!({"path": path})).await;
            assert_eq!(code, "invalid", "{kind} {path:?}");
        }
    }
    assert_eq!(
        fixture.failure("read", json!({"path": ""})).await,
        "invalid"
    );
    assert_eq!(
        fixture.failure("read", json!({"path": "sub"})).await,
        "invalid"
    );
    assert_eq!(
        fixture.failure("list", json!({"path": "a.txt"})).await,
        "invalid"
    );
    assert_eq!(
        fixture.failure("read", json!({"path": "gone.txt"})).await,
        "internal"
    );
    assert_eq!(fixture.failure("read", json!({})).await, "invalid");
    assert_eq!(
        fixture
            .failure("read", json!({"path": "a.txt", "reveal": "yes"}))
            .await,
        "invalid"
    );
    for commit in [
        "HEAD",
        "abc1234",
        "--output=x",
        &"A".repeat(40),
        tree.as_str(),
        &"0".repeat(40),
    ] {
        let code = fixture.failure("commit", json!({"commit": commit})).await;
        assert_eq!(code, "invalid", "commit {commit}");
        let code = fixture
            .failure("diff", json!({"path": "a.txt", "commit": commit}))
            .await;
        assert_eq!(code, "invalid", "diff commit {commit}");
    }
    let code = fixture
        .failure("diff", json!({"path": "a.txt", "old_path": "../x"}))
        .await;
    assert_eq!(code, "invalid");
    for skip in [json!(1_000_001), json!(-1), json!("x"), json!(1.5)] {
        let code = fixture.failure("history", json!({"skip": skip})).await;
        assert_eq!(code, "invalid", "skip {skip}");
    }
}

#[tokio::test]
async fn no_path_outside_the_root_is_ever_answered() {
    let mut fixture = Fixture::open().await;
    let repository = fixture.repository();
    let outside = fixture.outside();
    symlink(outside.join("key.txt"), repository.join("file-link")).expect("link");
    symlink(&outside, repository.join("folder-link")).expect("link");
    fs::create_dir(repository.join("hop")).expect("folder");
    symlink("../../outside", repository.join("hop/up")).expect("link");
    symlink("folder-link", repository.join("chain-a")).expect("link");
    symlink("chain-a/key.txt", repository.join("chain-b")).expect("link");
    symlink(".git", repository.join("data")).expect("link");
    symlink("a.txt", repository.join("inside-link")).expect("link");
    let absolute = outside.join("key.txt").to_string_lossy().into_owned();
    let mut answers = Vec::new();

    for path in [
        "file-link",
        "folder-link/key.txt",
        "folder-link/inner/deep.txt",
        "hop/up/key.txt",
        "chain-b",
        "../outside/key.txt",
        "sub/../../outside/key.txt",
        absolute.as_str(),
        ".git/config",
        "data/config",
    ] {
        answers.push(
            fixture
                .files("read", json!({"path": path, "reveal": true}))
                .await,
        );
        answers.push(
            fixture
                .files("diff", json!({"path": path, "reveal": true}))
                .await,
        );
    }
    for path in ["folder-link", "folder-link/inner", "hop/up", "data"] {
        answers.push(fixture.files("list", json!({"path": path})).await);
    }

    assert!(answers.len() >= 20);
    let mut outside_bytes = 0;
    for answer in &answers {
        assert_eq!(answer["ok"], false, "{answer}");
        assert!(
            ["refused", "invalid"].contains(&error_text(answer).0),
            "{answer}"
        );
        outside_bytes += answer.to_string().matches(OUTSIDE_MARKER).count();
    }
    assert_eq!(outside_bytes, 0);
    let control = fixture.ok("read", json!({"path": "inside-link"})).await;
    assert_eq!(text_of(&control), "one\n");
    let listed = fixture.ok("list", json!({"path": ""})).await;
    let targets: Vec<(String, Value)> = listed["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .filter(|entry| entry["kind"] == "link")
        .map(|entry| {
            (
                entry["name"].as_str().expect("name").to_owned(),
                entry["target"].clone(),
            )
        })
        .collect();
    assert!(
        targets.contains(&("file-link".to_owned(), json!("outside"))),
        "{targets:?}"
    );
    assert!(
        targets.contains(&("inside-link".to_owned(), json!("file"))),
        "{targets:?}"
    );
}

/// The files of the fixture that look secret, with the content of each.
const SECRET_NAMES: [&str; 10] = [
    ".env",
    ".env.local",
    "server.pem",
    "id_rsa",
    "api.key",
    ".npmrc",
    "db-secret.txt",
    "aws-credentials.json",
    "kubeconfig",
    "vpn.ovpn",
];

fn secret_content(index: usize, round: &str) -> String {
    format!("{SECRET_MARKER}-{index}-{round}\n")
}

#[tokio::test]
async fn secret_looking_files_send_no_content_until_one_reveal() {
    let mut fixture = Fixture::open().await;
    for (index, name) in SECRET_NAMES.iter().enumerate().take(5) {
        fixture.write(name, &secret_content(index, "old"));
    }
    commit_all(&fixture.repository(), "secrets");
    for (index, name) in SECRET_NAMES.iter().enumerate() {
        fixture.write(name, &secret_content(index, "new"));
    }
    let mut answers = Vec::new();

    answers.push(fixture.ok("list", json!({"path": ""})).await);
    let changes = fixture.ok("changes", json!({})).await;
    answers.push(changes.clone());
    for name in SECRET_NAMES {
        answers.push(fixture.ok("read", json!({"path": name})).await);
        answers.push(fixture.ok("diff", json!({"path": name})).await);
    }

    let listed = &answers[0]["entries"];
    for name in SECRET_NAMES {
        let entry = listed
            .as_array()
            .expect("entries")
            .iter()
            .find(|entry| entry["name"] == name)
            .expect(name);
        assert_eq!(entry["secret"], true, "{name}");
        let listed_change = changes["files"]
            .as_array()
            .expect("files")
            .iter()
            .find(|file| file["path"] == name)
            .expect(name);
        assert_eq!(listed_change["secret"], true, "{name}");
    }
    for answer in &answers {
        assert!(!answer.to_string().contains(SECRET_MARKER), "{answer}");
        assert!(answer["content_base64"].is_null(), "{answer}");
    }
    let states: Vec<&Value> = answers[2..].iter().map(|answer| &answer["state"]).collect();
    assert!(states.iter().all(|state| *state == "secret"), "{states:?}");

    let revealed = fixture
        .ok("read", json!({"path": SECRET_NAMES[3], "reveal": true}))
        .await;
    let other = fixture.ok("read", json!({"path": SECRET_NAMES[4]})).await;
    let again = fixture.ok("read", json!({"path": SECRET_NAMES[3]})).await;
    let revealed_diff = fixture
        .ok("diff", json!({"path": SECRET_NAMES[1], "reveal": true}))
        .await;

    assert_eq!(text_of(&revealed), secret_content(3, "new"));
    assert_eq!(other["state"], "secret");
    assert_eq!(again["state"], "secret");
    assert_eq!(revealed_diff["state"], "diff");
}

#[tokio::test]
async fn a_renamed_secret_file_is_secret_in_the_changes_and_in_the_commit() {
    let mut fixture = Fixture::open().await;
    fixture.write(
        ".env",
        "TOKEN=SECRETVALUE-rename\nSECOND=2\nTHIRD=3\nFOURTH=4\n",
    );
    commit_all(&fixture.repository(), "add env");
    git(&fixture.repository(), &["mv", ".env", "notes.txt"]);
    fixture.write(
        "notes.txt",
        "TOKEN=SECRETVALUE-renamed\nSECOND=2\nTHIRD=3\nFOURTH=4\n",
    );
    let rename = json!({"path": "notes.txt", "old_path": ".env"});

    let changes = fixture.ok("changes", json!({})).await;
    let hidden = fixture.ok("diff", rename.clone()).await;

    assert_eq!(changes["files"][0]["kind"], "renamed");
    assert_eq!(changes["files"][0]["secret"], true);
    assert_eq!(changes["files"][0]["old_path"], ".env");
    assert_eq!(
        (hidden["state"].clone(), hidden["content_base64"].clone()),
        (json!("secret"), Value::Null)
    );

    commit_all(&fixture.repository(), "rename env");
    let commit = fixture.head();
    let in_commit = json!({"path": "notes.txt", "old_path": ".env", "commit": commit});
    let detail = fixture.ok("commit", json!({"commit": commit})).await;
    let hidden = fixture.ok("diff", in_commit.clone()).await;
    let mut revealed_request = in_commit;
    revealed_request["reveal"] = json!(true);
    let revealed = fixture.ok("diff", revealed_request).await;

    assert_eq!(detail["files"][0]["secret"], true);
    assert_eq!(hidden["state"], "secret");
    assert!(hidden["content_base64"].is_null());
    assert_eq!(revealed["state"], "diff");
    assert!(text_of(&revealed).contains("+TOKEN=SECRETVALUE-renamed"));
}

#[tokio::test]
async fn the_agent_connection_refuses_a_files_request_and_records_it() {
    let office_folder = TempDir::new().expect("folder");
    let records = office_folder.path().display().to_string();
    let clis = FakeClis::new();
    clis.add(
        "claude",
        &format!(
            "env > {records}/env.part\nmv {records}/env.part {records}/env\n\
             {HELPER} agent-call agent.files.list '{{}}' > {records}/out 2> {records}/err\n\
             printf '%s' $? > {records}/code\nsleep 30\n"
        ),
    );
    let office = Office::open_with(clis).await;
    let mut client = office.client().await;
    let agent = client.create_agent(&office, "Ana", "claude", None).await;
    client.start(&agent).await;

    let code = wait_for("the helper's exit code", || {
        fs::read_to_string(office_folder.path().join("code"))
            .ok()
            .filter(|text| !text.is_empty())
    })
    .await;
    let out = wait_for("the helper's answer", || {
        fs::read_to_string(office_folder.path().join("out"))
            .ok()
            .filter(|text| !text.is_empty())
    })
    .await;

    let answer: Value = serde_json::from_str(out.trim()).expect("json");
    assert_eq!(code, "1", "{out}");
    assert_eq!(answer["error"]["code"], "refused", "{answer}");
    let actions = client
        .ask("agent.actions", json!({"agent_id": agent}))
        .await;
    let refused = actions["result"]
        .as_array()
        .expect("entries")
        .iter()
        .filter(|entry| entry["action"] == "refused")
        .count();
    assert_eq!(refused, 1, "{actions}");
}

/// The messages that reach `client` before the answer of one more request.
async fn events_before_an_answer(client: &mut Client) -> Vec<Value> {
    let id = client.send("agent.list", json!({})).await;
    let mut events = Vec::new();
    loop {
        let message = client.wire.receive_json().await;
        if message["id"] == id {
            return events;
        }
        events.push(message);
    }
}

#[tokio::test]
async fn reading_changes_no_status_publishes_no_event_and_records_no_action() {
    let mut fixture = Fixture::open().await;
    fixture.write("new.txt", "fresh\n");
    let mut observer = fixture.office.client().await;
    let before = fixture
        .client
        .ask("agent.actions", json!({"agent_id": fixture.agent}))
        .await;
    let listed_before = fixture.client.ask("agent.list", json!({})).await;

    fixture.ok("list", json!({"path": ""})).await;
    fixture.ok("read", json!({"path": "a.txt"})).await;
    fixture.ok("changes", json!({})).await;
    fixture.ok("diff", json!({"path": "new.txt"})).await;
    fixture.ok("history", json!({})).await;
    let head = fixture.head();
    fixture.ok("commit", json!({"commit": head})).await;
    fixture.failure("read", json!({"path": "../x"})).await;

    let after = fixture
        .client
        .ask("agent.actions", json!({"agent_id": fixture.agent}))
        .await;
    let listed_after = fixture.client.ask("agent.list", json!({})).await;
    let events = events_before_an_answer(&mut observer).await;
    assert_eq!(after["result"], before["result"]);
    assert_eq!(listed_after["result"], listed_before["result"]);
    assert!(events.is_empty(), "{events:?}");
}

#[tokio::test]
async fn a_large_folder_lists_reads_and_reports_changes_in_time() {
    let mut fixture = Fixture::open().await;
    let repository = fixture.repository();
    let folder_count = FILES_IN_FOLDER / ENTRIES_PER_FOLDER;
    for folder in 0..folder_count {
        let path = repository.join(format!("d{folder:02}"));
        fs::create_dir_all(&path).expect("folder");
        for file in 0..ENTRIES_PER_FOLDER {
            fs::write(
                path.join(format!("f{file:04}.txt")),
                format!("{folder} {file}\n"),
            )
            .expect("file");
        }
    }
    fs::write(repository.join("big.txt"), "x".repeat(MAX_CONTENT_BYTES)).expect("big file");
    commit_all(&repository, "many files");
    for file in 0..ENTRIES_PER_FOLDER {
        fs::write(repository.join(format!("d00/f{file:04}.txt")), "changed\n").expect("file");
    }

    let started = Instant::now();
    let listed = fixture.ok("list", json!({"path": "d01"})).await;
    let list_time = started.elapsed();
    let started = Instant::now();
    let big = fixture.ok("read", json!({"path": "big.txt"})).await;
    let read_time = started.elapsed();
    let started = Instant::now();
    let changes = fixture.ok("changes", json!({})).await;
    let changes_time = started.elapsed();

    assert_eq!(
        listed["entries"].as_array().expect("entries").len(),
        ENTRIES_PER_FOLDER
    );
    assert_eq!(listed["more"], 0);
    assert_eq!(big["state"], "text");
    assert_eq!(decoded(&big["content_base64"]).len(), MAX_CONTENT_BYTES);
    assert_eq!(
        changes["files"].as_array().expect("files").len(),
        ENTRIES_PER_FOLDER
    );
    assert!(list_time < LIST_LIMIT, "list took {list_time:?}");
    assert!(read_time < READ_LIMIT, "read took {read_time:?}");
    assert!(
        changes_time < CHANGES_LIMIT,
        "changes took {changes_time:?}"
    );
}

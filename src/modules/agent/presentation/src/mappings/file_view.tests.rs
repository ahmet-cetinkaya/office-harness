// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::Timestamp;

use super::*;

const AT: Timestamp = Timestamp::from_unix_millis(1_700_000_000_000);

#[test]
fn base64_matches_the_rfc_4648_vectors() {
    for (plain, encoded) in [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ] {
        assert_eq!(base64_encode(plain.as_bytes()), encoded, "{plain:?}");
    }
}

#[test]
fn base64_keeps_every_byte_value() {
    let every_byte: Vec<u8> = (0..=255).collect();

    let encoded = base64_encode(&every_byte);

    assert_eq!(&encoded[..8], "AAECAwQF");
    assert!(encoded.ends_with("P3+/w=="), "{encoded}");
    assert_eq!(base64_encode(&[0xfb, 0xff]), "+/8=");
}

fn entry(name: &str) -> FolderEntry {
    FolderEntry {
        name: name.to_owned(),
        kind: EntryKind::File,
        size: Some(1),
        secret: false,
        target: None,
    }
}

fn listing(entries: Vec<FolderEntry>, more: usize) -> FolderListing {
    FolderListing {
        root: "/work".to_owned(),
        path: String::new(),
        entries,
        more,
        read_at: AT,
    }
}

#[test]
fn a_listing_uses_the_field_names_of_the_contract() {
    let mut link = entry("alias");
    link.kind = EntryKind::Link;
    link.size = None;
    link.secret = true;
    link.target = Some(LinkTarget::Outside);

    let json = listing_json(&listing(vec![entry("a.txt"), link], 3));

    assert_eq!(json["root"], "/work");
    assert_eq!(json["path"], "");
    assert_eq!(json["more"], 3);
    assert_eq!(json["read_at"], 1_700_000_000_000_i64);
    assert_eq!(
        json["entries"][0],
        json!({"name": "a.txt", "kind": "file", "size": 1, "secret": false, "target": null})
    );
    assert_eq!(
        json["entries"][1],
        json!({"name": "alias", "kind": "link", "size": null, "secret": true, "target": "outside"})
    );
}

#[test]
fn names_full_of_control_characters_stop_at_the_budget_and_count_in_more() {
    // Each escaped control character is six bytes of JSON.
    let name = "\u{1}".repeat(255);
    let entries: Vec<FolderEntry> = (0..1_000).map(|_| entry(&name)).collect();

    let json = listing_json(&listing(entries, 0));

    let kept = json["entries"].as_array().expect("entries").len();
    let left_out = usize::try_from(json["more"].as_u64().expect("more")).expect("count");
    assert!(kept < 1_000 && kept > 0);
    assert_eq!(kept + left_out, 1_000);
    assert!(json.to_string().len() < LIST_BUDGET_BYTES + 1_024);
}

#[test]
fn a_full_listing_of_plain_names_is_not_cut() {
    let entries: Vec<FolderEntry> = (0..1_000)
        .map(|index| entry(&format!("file-{index}")))
        .collect();

    let json = listing_json(&listing(entries, 0));

    assert_eq!(json["entries"].as_array().expect("entries").len(), 1_000);
    assert_eq!(json["more"], 0);
}

#[test]
fn content_and_diff_carry_base64_or_null() {
    let content = FileContent {
        path: "a.txt".to_owned(),
        state: ContentState::Text,
        size: 3,
        bytes: Some(b"foo".to_vec()),
        read_at: AT,
    };
    let secret = FileDiff {
        path: "a.txt".to_owned(),
        state: DiffState::Secret,
        size: None,
        bytes: None,
    };

    assert_eq!(
        content_json(&content),
        json!({"path": "a.txt", "state": "text", "size": 3, "content_base64": "Zm9v", "read_at": 1_700_000_000_000_i64})
    );
    assert_eq!(
        diff_json(&secret),
        json!({"path": "a.txt", "state": "secret", "size": null, "content_base64": null})
    );
}

#[test]
fn history_says_more_when_the_budget_left_a_commit_out() {
    let commit = |subject: &str| CommitSummary {
        id: "a".repeat(40),
        short_id: "aaaaaaa".to_owned(),
        subject: subject.to_owned(),
        author: "Ana".to_owned(),
        time: AT,
    };
    let heavy = "\u{1}".repeat(200);
    let page = CommitPage {
        repository: true,
        commits: (0..50).map(|_| commit(&heavy.repeat(20))).collect(),
        more: false,
    };

    let json = history_json(&page);

    assert_eq!(json["more"], true);
    assert!(json["commits"].as_array().expect("commits").len() < 50);
    assert_eq!(
        history_json(&CommitPage {
            repository: true,
            commits: vec![commit("one")],
            more: false
        })["more"],
        false
    );
}

#[test]
fn a_commit_answer_budgets_its_files_after_its_message() {
    let file = |index: usize| ChangedFile {
        path: format!("{}{index}", "\u{1}".repeat(255)),
        kind: ChangeKind::Renamed,
        old_path: Some("old".to_owned()),
        secret: true,
    };
    let detail = CommitDetail {
        id: "a".repeat(40),
        parents: vec!["b".repeat(40)],
        author: "Ana".to_owned(),
        email: "ana@example.com".to_owned(),
        time: AT,
        message: "\u{1}".repeat(65_000),
        files: (0..1_000).map(file).collect(),
        more: 0,
    };

    let json = commit_json(&detail);

    assert!(
        json.to_string().len() < 1024 * 1024,
        "{}",
        json.to_string().len()
    );
    assert!(json["more"].as_u64().expect("more") > 0);
    assert_eq!(json["files"][0]["kind"], "renamed");
    assert_eq!(json["files"][0]["old_path"], "old");
    assert_eq!(json["parents"][0], "b".repeat(40));
}

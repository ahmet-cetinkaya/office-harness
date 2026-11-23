// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn parse(text: &str) -> Result<RelativePath, AgentError> {
    RelativePath::parse_for("path", text, false)
}

#[test]
fn a_plain_path_keeps_its_parts() {
    let path = parse("src/lib/a.rs").expect("a valid path");
    assert_eq!(path.parts(), ["src", "lib", "a.rs"]);
    assert_eq!(path.name(), Some("a.rs"));
    assert_eq!(path.to_text(), "src/lib/a.rs");
}

#[test]
fn every_refused_form_names_the_field() {
    for text in [
        "",
        "/etc/passwd",
        "a//b",
        "a/",
        "./a",
        "a/./b",
        "..",
        "a/../b",
        "a/b/..",
        "a\0b",
    ] {
        let error = parse(text).expect_err(text);
        assert!(error.to_string().starts_with("path "), "{text}: {error}");
    }
}

#[test]
fn backslash_and_colon_are_refused_only_on_windows() {
    for text in ["a\\b", "C:/x", "a:b"] {
        assert!(
            RelativePath::parse_for("path", text, true).is_err(),
            "{text}"
        );
        assert!(
            RelativePath::parse_for("path", text, false).is_ok(),
            "{text}"
        );
    }
}

#[test]
fn only_the_folder_form_accepts_the_root() {
    assert!(RelativePath::parse("path", "").is_err());
    let root = RelativePath::parse_folder("path", "").expect("the root");
    assert!(root.is_root());
    assert_eq!(root, RelativePath::root());
    assert_eq!(root.name(), None);
    assert!(RelativePath::parse_folder("path", "a/../b").is_err());
}

#[test]
fn every_name_of_the_secret_list_is_secret_in_any_case() {
    let names = [
        ".env",
        ".env.local",
        "a.pem",
        "a.key",
        "a.p12",
        "a.pfx",
        "a.jks",
        "a.keystore",
        "a.kdbx",
        "a.ppk",
        "a.tfstate",
        "id_rsa",
        "id_rsa.pub",
        "id_dsa",
        "id_ecdsa",
        "id_ed25519",
        ".netrc",
        ".npmrc",
        ".pypirc",
        ".pgpass",
        ".git-credentials",
        ".htpasswd",
        "kubeconfig",
        "client.ovpn",
        "release.asc",
        "my-secret-notes.txt",
        "aws_credentials",
    ];
    for name in names {
        assert!(is_secret_looking(name), "{name}");
        assert!(is_secret_looking(&name.to_uppercase()), "{name} upper");
    }
}

#[test]
fn near_misses_are_not_secret() {
    for name in [
        "environment.txt",
        "keys.md",
        ".envrc",
        "env",
        "monkey",
        "kubeconfig.md",
        "notes.ascii",
        "pem",
        "my_id_rsa",
        "README.md",
    ] {
        assert!(!is_secret_looking(name), "{name}");
    }
}

#[test]
fn a_zero_byte_in_the_first_8000_bytes_makes_a_file_binary() {
    let mut content = vec![b'a'; 9_000];
    content[BINARY_PREFIX_BYTES - 1] = 0;
    assert!(is_binary(&content));
    content[BINARY_PREFIX_BYTES - 1] = b'a';
    content[BINARY_PREFIX_BYTES] = 0;
    assert!(!is_binary(&content));
    assert!(!is_binary(b""));
    assert!(is_binary(&[0]));
}

#[test]
fn the_limits_are_the_specified_numbers() {
    assert_eq!(MAX_CONTENT_BYTES, 524_288);
    assert_eq!(LIST_BUDGET_BYTES, 786_432);
    assert_eq!(MAX_STATUS_OUTPUT_BYTES, 8_388_608);
}

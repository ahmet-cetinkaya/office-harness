// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn server_file(protocol: u16, version: &str) -> ServerFile {
    ServerFile {
        port: 4000,
        protocol,
        version: version.to_owned(),
    }
}

#[test]
fn parses_the_file_the_server_writes() {
    let text = r#"{"port":4000,"pid":9,"protocol":1,"version":"0.1.0","start_nonce":"ab"}"#;

    assert_eq!(
        ServerFile::parse(text).expect("valid"),
        server_file(1, "0.1.0")
    );
}

#[test]
fn rejects_text_that_is_not_a_server_file() {
    assert!(ServerFile::parse("{\"port\":\"x\"}").is_err());
    assert!(ServerFile::parse("").is_err());
}

#[test]
fn the_same_protocol_and_version_are_compatible() {
    assert_eq!(
        server_file(PROTOCOL_VERSION, "0.1.0").incompatibility("0.1.0"),
        None
    );
}

#[test]
fn another_version_can_be_restarted() {
    let found = server_file(PROTOCOL_VERSION, "0.0.9").incompatibility("0.1.0");

    assert_eq!(
        found,
        Some(Incompatibility {
            server_version: "0.0.9".to_owned(),
            app_version: "0.1.0".to_owned(),
            can_restart: true,
        })
    );
}

#[test]
fn another_protocol_cannot_be_restarted() {
    let found = server_file(PROTOCOL_VERSION + 1, "0.1.0").incompatibility("0.1.0");

    assert_eq!(found.map(|found| found.can_restart), Some(false));
}

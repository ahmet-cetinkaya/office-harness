// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::Value;

use super::*;

#[test]
fn the_file_holds_the_contract_fields() {
    let folder = tempfile::tempdir().expect("folder");
    let path = folder.path().join("server.json");

    write_server_file(&path, 4242, "abcd").expect("write");

    let text = std::fs::read_to_string(&path).expect("read");
    let file: Value = serde_json::from_str(&text).expect("json");
    assert_eq!(file["port"], 4242);
    assert_eq!(file["pid"], std::process::id());
    assert_eq!(file["protocol"], 1);
    assert_eq!(file["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(file["start_nonce"], "abcd");
}

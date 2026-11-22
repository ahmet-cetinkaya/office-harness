// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! 0008 PB-5 (and 0001 FR-005): the code that makes Needs you and the
//! notifications sends no office data off the machine. The requests are wire
//! requests on the loopback connection that exists already, so the guard is
//! that these crates hold no way to open another one: no dependency outside
//! the list below, and no network or process call in their sources.

use std::fs;
use std::path::{Path, PathBuf};

const NOTIFICATION_LAYERS: [&str; 4] = ["domain", "application", "infrastructure", "presentation"];
/// The dependencies the notification crates may have: the office's own crates,
/// the wire format, SQLite, and plain value libraries. None opens a socket.
const ALLOWED_DEPENDENCIES: [&str; 9] = [
    "kernel",
    "wire",
    "serde",
    "serde_json",
    "thiserror",
    "uuid",
    "rusqlite",
    "notification-domain",
    "notification-application",
];
const BRIDGES_OF_THE_QUEUE: [&str; 2] = ["needs_you_bridge.rs", "notification_bridge.rs"];
/// What a source needs to reach outside its own process.
const OUTBOUND_CALLS: [&str; 8] = [
    "std::net",
    "tokio::net",
    "TcpStream",
    "UdpSocket",
    "std::process",
    "tokio::process",
    "reqwest",
    "hyper",
];

fn notification_folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../modules/notification")
}

/// The dependency names under `[dependencies]` of a manifest.
fn dependencies_of(manifest: &Path) -> Vec<String> {
    let text = fs::read_to_string(manifest).expect("manifest");
    text.lines()
        .skip_while(|line| line.trim() != "[dependencies]")
        .skip(1)
        .take_while(|line| !line.starts_with('['))
        .filter_map(|line| line.split_once('='))
        .map(|(name, _)| name.trim().trim_end_matches(".workspace").to_owned())
        .collect()
}

fn rust_sources_under(folder: &Path) -> Vec<PathBuf> {
    let mut sources = Vec::new();
    for entry in fs::read_dir(folder).expect("folder") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            sources.extend(rust_sources_under(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            sources.push(path);
        }
    }
    sources
}

#[test]
fn the_notification_crates_depend_on_nothing_that_opens_a_connection() {
    for layer in NOTIFICATION_LAYERS {
        let manifest = notification_folder().join(layer).join("Cargo.toml");
        let dependencies = dependencies_of(&manifest);
        assert!(!dependencies.is_empty(), "{layer}: no dependency was read");
        for dependency in dependencies {
            assert!(
                ALLOWED_DEPENDENCIES.contains(&dependency.as_str()),
                "{layer} depends on {dependency}, which the office has not cleared as local"
            );
        }
    }
}

#[test]
fn the_sources_of_the_queue_call_no_network_or_process_api() {
    let mut sources: Vec<PathBuf> = NOTIFICATION_LAYERS
        .iter()
        .flat_map(|layer| rust_sources_under(&notification_folder().join(layer).join("src")))
        .collect();
    let bridges = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/bridges");
    sources.extend(BRIDGES_OF_THE_QUEUE.iter().map(|name| bridges.join(name)));
    assert!(
        sources.len() > BRIDGES_OF_THE_QUEUE.len(),
        "no source found"
    );

    for source in sources {
        let text = fs::read_to_string(&source).expect("source");
        for call in OUTBOUND_CALLS {
            assert!(!text.contains(call), "{} mentions {call}", source.display());
        }
    }
}

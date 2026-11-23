// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(dead_code, reason = "each test binary uses a part of the helpers")]

pub mod fake_cli;
pub mod office_client;
pub mod running_office;
pub mod server_process;
pub mod wire_client;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use agent_domain::entities::Agent;
use agent_domain::value_objects::{AgentCommand, DisplayName, MachineId};
use kernel::{AgentId, Timestamp};
use office_harnessd::config::Config;
use tempfile::TempDir;

pub fn config_in(folder: &TempDir) -> Config {
    Config {
        data_folder: folder.path().join("office"),
    }
}

pub fn agent_named(machine_id: MachineId, name: &str) -> Agent {
    Agent::new(
        AgentId::generate(),
        DisplayName::parse("name", name).expect("name"),
        AgentCommand::parse("claude").expect("command"),
        Some(std::env::temp_dir()),
        None,
        machine_id,
        Timestamp::from_unix_millis(1_700_000_000_000),
    )
    .expect("agent")
}

/// Every folder and file under `folders`, with the bytes of each file: what a
/// test compares before and after to show that nothing was written.
pub fn entries_under(folders: &[&Path]) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut entries = BTreeMap::new();
    let mut pending: Vec<PathBuf> = folders.iter().map(|folder| folder.to_path_buf()).collect();
    while let Some(folder) = pending.pop() {
        for entry in fs::read_dir(&folder).expect("a readable folder") {
            let path = entry.expect("a folder entry").path();
            if path.is_dir() {
                entries.insert(path.clone(), None);
                pending.push(path);
            } else {
                let contents = fs::read(&path).expect("a readable file");
                entries.insert(path, Some(contents));
            }
        }
    }
    entries
}

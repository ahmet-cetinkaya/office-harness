// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(windows)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! A stop on Windows must not wait for the grace period. These tests run on
//! the Windows runner only, with the default 10 second grace: an npm shim
//! starts its interpreter with no `cmd.exe`, and a batch file that is not an
//! npm shim has its "Terminate batch job (Y/N)?" question answered.

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use support::fake_cli::FakeClis;
use support::office_client::{Client, Office, recording_body, wait_for, wait_for_records};

/// Well under the 10 second grace, and well over what a clean stop takes.
const FAST_STOP_LIMIT: Duration = Duration::from_secs(5);
/// How long a plain batch file keeps running, if nothing ends it.
const BATCH_KEEP_ALIVE_PINGS: u32 = 32;
/// The address that `ping` prints when it starts, in any language. The batch
/// file blocks in `ping`, so seeing it proves that the interrupt key will
/// reach a running program.
const KEEP_ALIVE_ADDRESS: &str = "127.0.0.1";
const SHIM_RECORD_FILE: &str = "shim-record.json";
const UNIQUE_MARKER: &str = "--office-shim-marker-7f3a91";
const NODE_FILE: &str = "node.exe";
const SHELL_NAME: &str = "cmd.exe";

/// The npm shim of the newer form, which finds `node` on the `PATH`.
const NPM_SHIM: &str = "@ECHO off\r\nGOTO start\r\n:find_dp0\r\nSET dp0=%~dp0\r\nEXIT /b\r\n:start\r\nSETLOCAL\r\nCALL :find_dp0\r\n\r\nSET \"_prog=node\"\r\nendLocal & goto #_undefined_# 2>NUL || title %COMSPEC% & \"%_prog%\"  \"%dp0%\\node_modules\\fake\\cli.js\" %*\r\n";

/// The folder of the `node.exe` that the test process itself can find.
fn node_folder() -> PathBuf {
    let search_path = std::env::var_os("PATH").expect("PATH");
    std::env::split_paths(&search_path)
        .find(|folder| folder.join(NODE_FILE).is_file())
        .expect("node.exe on the PATH of the test process")
}

/// The script writes what it received, prints, and waits until it is ended.
fn write_fake_script(clis: &FakeClis, records: &Path) {
    let record_path = records.join(SHIM_RECORD_FILE);
    let script_folder = clis.tool_folder().join("node_modules").join("fake");
    fs::create_dir_all(&script_folder).expect("script folder");
    let target = serde_json::to_string(&record_path.display().to_string()).expect("path");
    let script = format!(
        "if (process.argv.slice(2, 4).join(' ') === 'auth status') process.exit(0);\nif (process.argv[2] === '--version') {{ console.log('2.1.322 (Claude Code)'); process.exit(0); }}\nconst fs = require('node:fs');\nconst target = {target};\nfs.writeFileSync(target + '.part', JSON.stringify({{ argv: process.argv.slice(2), pid: process.pid, ppid: process.ppid }}));\nfs.renameSync(target + '.part', target);\nconsole.log('ready');\nsetInterval(() => {{}}, 1000);\n"
    );
    fs::write(script_folder.join("cli.js"), script).expect("script");
    fs::write(clis.tool_folder().join("claude.cmd"), NPM_SHIM).expect("shim");
}

async fn wait_for_shim_record(records: &Path) -> Value {
    let path = records.join(SHIM_RECORD_FILE);
    let text = wait_for("the script record", || fs::read_to_string(&path).ok()).await;
    serde_json::from_str(&text).expect("record json")
}

fn process_id_of(record: &Value, key: &str) -> u32 {
    u32::try_from(record[key].as_u64().expect("process id")).expect("process id fits")
}

/// `tasklist` prints the id in quotes in each row it finds, and no row when
/// the process is gone.
fn is_process_running(process_id: u32) -> bool {
    let listing = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {process_id}"), "/FO", "CSV", "/NH"])
        .output()
        .expect("tasklist");
    String::from_utf8_lossy(&listing.stdout).contains(&format!("\"{process_id}\""))
}

fn process_name_of(process_id: u32) -> String {
    let query = format!("(Get-CimInstance Win32_Process -Filter 'ProcessId={process_id}').Name");
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &query])
        .output()
        .expect("powershell");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// Waits until the batch file runs `ping`. The interrupt key that arrives
/// between `echo` and the start of `ping` is lost, and the batch file then
/// keeps running until the grace period ends, so a stop must not start sooner.
async fn wait_until_the_batch_blocks_in_ping(client: &mut Client, agent: &str) {
    let attached = client
        .ask(
            "terminal.attach",
            json!({"agent_id": agent, "cols": 80, "rows": 24}),
        )
        .await;
    assert_eq!(attached["ok"], true, "{attached}");
    let mut seen = String::new();
    while !seen.contains(KEEP_ALIVE_ADDRESS) {
        let frame = client.wire.receive_binary().await;
        seen.push_str(&String::from_utf8_lossy(&frame));
    }
}

/// Stops the agent with the default grace and returns how long the request
/// took.
async fn timed_stop(office: &Office, agent: &str) -> Duration {
    let mut client = office.client().await;
    let started = Instant::now();
    let stopped = client.ask("agent.stop", json!({"agent_id": agent})).await;
    let elapsed = started.elapsed();
    assert_eq!(stopped["ok"], true, "{stopped}");
    elapsed
}

#[tokio::test]
async fn an_npm_shim_starts_its_interpreter_directly_and_a_stop_is_fast() {
    let clis = FakeClis::new().with_path_folder(node_folder());
    let office = Office::open_with(clis).await;
    write_fake_script(&office.clis, office.records_path());
    let mut client = office.client().await;
    let agent = client
        .create_agent(
            &office,
            "Ana",
            "claude",
            Some(json!({"flags": [UNIQUE_MARKER]})),
        )
        .await;
    client.start(&agent).await;
    let record = wait_for_shim_record(office.records_path()).await;
    let arguments = record["argv"].as_array().expect("argv");
    assert!(
        arguments.iter().any(|argument| argument == UNIQUE_MARKER),
        "{record}"
    );
    let script_id = process_id_of(&record, "pid");
    assert_ne!(
        process_name_of(process_id_of(&record, "ppid")),
        SHELL_NAME,
        "no cmd.exe stands between the server and the script"
    );

    let elapsed = timed_stop(&office, &agent).await;

    assert!(elapsed < FAST_STOP_LIMIT, "the stop took {elapsed:?}");
    wait_for("the script process to end", || {
        (!is_process_running(script_id)).then_some(())
    })
    .await;
    office.serving.stop().await;
}

#[tokio::test]
async fn a_batch_file_that_is_not_an_npm_shim_is_stopped_by_answering_its_question() {
    let office = Office::open().await;
    office.clis.add(
        "claude",
        &recording_body(
            office.records_path(),
            &format!("echo ready\nping -n {BATCH_KEEP_ALIVE_PINGS} {KEEP_ALIVE_ADDRESS}"),
        ),
    );
    let mut client = office.client().await;
    let agent = client.create_agent(&office, "Ana", "claude", None).await;
    client.start(&agent).await;
    wait_for_records(office.records_path(), 1).await;
    wait_until_the_batch_blocks_in_ping(&mut client, &agent).await;

    let elapsed = timed_stop(&office, &agent).await;

    assert!(
        elapsed < FAST_STOP_LIMIT,
        "the stop took {elapsed:?}; the batch question was not answered"
    );
    office.serving.stop().await;
}

// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(windows)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! A one-shot CI probe, not a product test: does `ConPTY` forward the BEL
//! byte (0x07) that a child writes, or does it drop it? The tests always
//! panic with a hexdump of the raw bytes read from the master side, so the
//! CI log itself is the harvested evidence. The PTY setup is the one
//! `pty_host` uses: `native_pty_system`, one `openpty`, the child on the
//! slave, and raw reads from the cloned master reader, with no runtime
//! layers in between.

use std::fmt::Write as _;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

use portable_pty::{Child, CommandBuilder, PtySize, native_pty_system};

const READ_LIMIT: Duration = Duration::from_secs(20);
const EXIT_WAIT_LIMIT: Duration = Duration::from_secs(5);
const EXIT_POLL: Duration = Duration::from_millis(50);
const BEL: u8 = 0x07;
/// The fixture file holds exactly this, so the raw stream can be judged
/// against known landmarks. The interpreter of the child is `cmd /d /c
/// type`, which starts fast and copies the bytes of the file to its console
/// untouched.
const CHILD_OUTPUT_START: &str = "BEFORE";
const CHILD_OUTPUT_END: &str = "AFTER";
/// `/d` skips the interpreter's auto-run commands and `/c` runs the one
/// command and ends, so no prompt or banner joins the stream.
const CHILD_ARGUMENTS: [&str; 3] = ["/d", "/c", "type"];

fn windows_version() -> String {
    let output = std::process::Command::new("cmd")
        .args(["/c", "ver"])
        .output()
        .expect("cmd /c ver");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// A file in a fresh temp folder holding the landmarks with the BEL between
/// them, so the child has bytes to copy and no argument of the probe can be
/// mangled by the interpreter's quoting.
fn write_fixture() -> tempfile::TempDir {
    let folder = tempfile::TempDir::new().expect("temp folder");
    let mut file = std::fs::File::create(folder.path().join("bel.txt")).expect("fixture file");
    file.write_all(format!("{CHILD_OUTPUT_START}{BEL}{CHILD_OUTPUT_END}").as_bytes())
        .and_then(|()| file.flush())
        .expect("write fixture");
    folder
}

/// Reads the master side on a helper thread, because a read with no data
/// coming blocks and the probe must return within `READ_LIMIT` even when
/// the child never closes the pipe. An abandoned thread dies with the
/// process.
fn read_master_within(reader: Box<dyn Read + Send>, limit: Duration) -> Vec<u8> {
    let (chunk_sender, chunk_receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = reader;
        let mut chunk = [0_u8; 4096];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(count) => {
                    if chunk_sender.send(chunk[..count].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    let mut received = Vec::new();
    let deadline = Instant::now() + limit;
    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        match chunk_receiver.recv_timeout(remaining) {
            Ok(bytes) => received.extend_from_slice(&bytes),
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => break,
        }
    }
    received
}

/// Ends the child if it is still alive after `EXIT_WAIT_LIMIT`, and reports
/// how it ended, so a stuck child cannot hang the CI job past the read. An
/// `NTSTATUS` status is shown in hex, because a console that closed under a
/// running child reports codes like `0xC000013A`.
fn exit_report(child: &mut (dyn Child + Send + Sync)) -> String {
    let deadline = Instant::now() + EXIT_WAIT_LIMIT;
    loop {
        match child.try_wait().expect("try_wait") {
            Some(status) => return status_report(status.exit_code()),
            None if Instant::now() >= deadline => break,
            None => std::thread::sleep(EXIT_POLL),
        }
    }
    let mut killer = child.clone_killer();
    let _ = killer.kill();
    let status = child.wait().expect("wait after kill");
    format!(
        "was killed while running, then {}",
        status_report(status.exit_code())
    )
}

fn status_report(code: u32) -> String {
    if code > u32::from(u16::MAX) {
        format!("exited with NTSTATUS 0x{code:08X}")
    } else {
        format!("exited with code {code}")
    }
}

/// Starts the child that copies the fixture file with `type` on the slave of
/// a fresh `ConPTY` pair, the same `openpty` - `spawn_command` -
/// `try_clone_reader` sequence as `pty_host`, and returns the raw master
/// bytes and how the child ended.
fn read_child_output(fixture: &Path) -> (Vec<u8>, String) {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("openpty");
    let mut builder = CommandBuilder::new("cmd.exe");
    builder.args(CHILD_ARGUMENTS);
    builder.arg(fixture);
    builder.cwd(std::env::temp_dir());
    let mut child = pair.slave.spawn_command(builder).expect("spawn child");
    // The child holds the slave side now, as in `pty_host`.
    drop(pair.slave);
    let reader = pair.master.try_clone_reader().expect("clone master reader");

    let received = read_master_within(reader, READ_LIMIT);
    // The master stays alive until the reads are done, as in `pty_host`.
    drop(pair.master);
    let ending = exit_report(child.as_mut());
    (received, ending)
}

fn hexdump(bytes: &[u8]) -> String {
    let mut dump = String::new();
    for (offset, line) in bytes.chunks(16).enumerate() {
        let mut hex = String::new();
        for byte in line {
            let _ = write!(hex, "{byte:02x} ");
        }
        let ascii: String = line
            .iter()
            .map(|&byte| {
                if (0x20..0x7f).contains(&byte) {
                    byte as char
                } else {
                    '.'
                }
            })
            .collect();
        let _ = writeln!(dump, "{:04x}  {hex:<48} {ascii}", offset * 16);
    }
    dump
}

/// The failure text is the evidence: verdict, Windows image, and every raw
/// byte, whatever the outcome.
fn panic_with_evidence(label: &str, received: &[u8], ending: &str) -> ! {
    let has_bell = received.contains(&BEL);
    let bell_offset = received.iter().position(|&byte| byte == BEL);
    let verdict = if has_bell { "PRESENT" } else { "ABSENT" };
    let text = String::from_utf8_lossy(received);
    let mut message = format!(
        "CONPTY_BEL_PROBE {label} verdict: BEL byte (0x07) is {verdict} in the raw master stream. \
         windows: {}. child {ending}. bytes read: {}. landmarks BEFORE: {}, AFTER: {}. \
         first 0x07 at offset: {bell_offset:?}. text: {text:?}. \
         hexdump (offset hex ascii):",
        windows_version(),
        received.len(),
        text.contains(CHILD_OUTPUT_START),
        text.contains(CHILD_OUTPUT_END),
    );
    let _ = write!(message, "\n{}", hexdump(received));
    panic!("{message}");
}

fn exe_folder() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(Path::to_path_buf)
}

/// `LoadLibraryW("conpty.dll")` looks next to the test binary, then in the
/// system folders, then along `PATH`. The system folders never carry a
/// `conpty.dll`, so those two places cover what the loader would find.
fn resolvable_conpty_dll() -> Option<PathBuf> {
    let next_to_test = exe_folder()?.join("conpty.dll");
    if next_to_test.is_file() {
        return Some(next_to_test);
    }
    let search_path = std::env::var_os("PATH")?;
    std::env::split_paths(&search_path)
        .map(|folder| folder.join("conpty.dll"))
        .find(|candidate| candidate.is_file())
}

#[test]
fn probe_conpty_forwards_the_bell_byte() {
    let fixture = write_fixture();
    let (received, ending) = read_child_output(&fixture.path().join("bel.txt"));
    panic_with_evidence("stock", &received, &ending);
}

/// portable-pty prefers a `conpty.dll` next to the test binary over the
/// kernel32 exports, so a sideloaded newer console would be probed the same
/// way through the same `native_pty_system` call. No runner ships one and
/// this experiment adds no binary downloads, so with no DLL resolvable the
/// test returns without failing the job.
#[test]
fn probe_sideloaded_conpty_dll_forwards_the_bell_byte() {
    let Some(dll) = resolvable_conpty_dll() else {
        return;
    };
    let fixture = write_fixture();
    let (received, ending) = read_child_output(&fixture.path().join("bel.txt"));
    let label = format!("sideloaded dll {}", dll.display());
    panic_with_evidence(&label, &received, &ending);
}

// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(windows)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! A one-shot CI probe, not a product test: does `ConPTY` forward the `BEL`
//! byte (0x07) that a child writes, or does it drop it? An earlier round
//! showed that this runner's `ConPTY` emits only its init cursor query
//! (`ESC[6n`) to the master reader and no child bytes at all, so this file
//! probes twice. The raw probe answers that query on the writer, and the
//! product probe drives the real `PortableTerminalRuntime` the line tests
//! use, with a raw-byte tap in place of the usual subscriber. Both tests
//! always panic with a hexdump of every byte received, so the CI log is the
//! harvested evidence.

mod support;

use std::fmt::Write as _;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use kernel::AgentId;
use portable_pty::{Child, CommandBuilder, PtySize, native_pty_system};
use terminal_application::common::interfaces::{OutputSubscriber, TerminalRuntime};
use terminal_application::common::models::{DeliveryStatus, LiveOutput};

const READ_LIMIT: Duration = Duration::from_secs(20);
const EXIT_WAIT_LIMIT: Duration = Duration::from_secs(5);
const EXIT_POLL: Duration = Duration::from_millis(50);
const CHILD_EXIT_LIMIT: Duration = Duration::from_secs(25);
const CHILD_EXIT_POLL: Duration = Duration::from_millis(100);
const AFTER_EXIT_GRACE: Duration = Duration::from_secs(2);
const BEL: u8 = 0x07;
/// `ConPTY` opens every session with this cursor-position query, and the
/// earlier probe round showed the runner's console holds the child's output
/// back while it stands unanswered.
const CURSOR_QUERY: [u8; 4] = [0x1b, 0x5b, 0x36, 0x6e];
/// The answer of a cursor at row 1, column 1.
const CURSOR_REPLY: [u8; 6] = [0x1b, 0x5b, 0x31, 0x3b, 0x31, 0x52];
/// The fixture file holds exactly this, so the raw stream can be judged
/// against known landmarks.
const CHILD_OUTPUT_START: &str = "BEFORE";
const CHILD_OUTPUT_END: &str = "AFTER";
const FIXTURE_NAME: &str = "bel.txt";
/// `/d` skips the interpreter's auto-run commands and `/c` runs the one
/// command and ends, so no prompt or banner joins the stream.
const CHILD_ARGUMENTS: [&str; 3] = ["/d", "/c", "type"];

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn windows_version() -> String {
    let output = std::process::Command::new("cmd")
        .args(["/c", "ver"])
        .output()
        .expect("cmd /c ver");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// A file in a fresh temp folder holding the landmarks with the `BEL`
/// between
/// them, so the child has bytes to copy and no argument of the probe can be
/// mangled by the interpreter's quoting.
fn write_fixture() -> tempfile::TempDir {
    let folder = tempfile::TempDir::new().expect("temp folder");
    let mut file = std::fs::File::create(folder.path().join(FIXTURE_NAME)).expect("fixture file");
    file.write_all(format!("{CHILD_OUTPUT_START}{BEL}{CHILD_OUTPUT_END}").as_bytes())
        .and_then(|()| file.flush())
        .expect("write fixture");
    folder
}

/// Collects the master bytes while watching for the cursor query, and
/// answers it on the writer once, so the console starts delivering what the
/// child writes. A query split across two reads is still found, through the
/// carried tail. The read runs on a helper thread, because a read with no
/// data coming blocks and the probe must return within `READ_LIMIT` even
/// when the child never closes the pipe; an abandoned thread dies with the
/// process.
fn read_master_with_handshake(
    reader: Box<dyn Read + Send>,
    writer: &mut (dyn Write + Send),
    limit: Duration,
) -> (Vec<u8>, bool, bool) {
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
    let mut carried = Vec::new();
    let mut query_seen = false;
    let mut reply_sent = false;
    let deadline = Instant::now() + limit;
    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        match chunk_receiver.recv_timeout(remaining) {
            Ok(bytes) => {
                carried.extend_from_slice(&bytes);
                if !reply_sent
                    && carried
                        .windows(CURSOR_QUERY.len())
                        .any(|part| part == CURSOR_QUERY)
                {
                    query_seen = true;
                    reply_sent = writer
                        .write_all(&CURSOR_REPLY)
                        .and_then(|()| writer.flush())
                        .is_ok();
                }
                let tail = CURSOR_QUERY.len() - 1;
                if carried.len() > tail {
                    carried.drain(..carried.len() - tail);
                }
                received.extend_from_slice(&bytes);
            }
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => break,
        }
    }
    (received, query_seen, reply_sent)
}

/// Ends the child if it is still alive after `EXIT_WAIT_LIMIT`, and reports
/// how it ended, so a stuck child cannot hang the CI job past the read. An
/// An `NTSTATUS` status is shown in hex, because a console that closed under a
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
/// `try_clone_reader` sequence as `pty_host`. The master writer is taken too
/// and answers the cursor query. Returns the raw master bytes, how the child
/// ended, and what became of the handshake.
fn read_raw_child_output(fixture: &Path) -> (Vec<u8>, String, bool, bool) {
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
    let mut writer = pair.master.take_writer().expect("take master writer");

    let (received, query_seen, reply_sent) =
        read_master_with_handshake(reader, writer.as_mut(), READ_LIMIT);
    // The master stays alive until the reads are done, as in `pty_host`.
    drop(pair.master);
    let ending = exit_report(child.as_mut());
    (received, ending, query_seen, reply_sent)
}

/// Every master byte the product runtime delivers, kept raw.
#[derive(Default)]
struct MasterTap {
    collected: Mutex<Vec<u8>>,
}

impl OutputSubscriber for MasterTap {
    fn deliver(&self, output: LiveOutput) -> DeliveryStatus {
        if let LiveOutput::Bytes(chunk) = &output {
            locked(&self.collected).extend_from_slice(chunk);
        }
        DeliveryStatus::Accepted
    }
}

/// Drives the real runtime the line tests use, with the same fixture child,
/// and taps the raw master bytes through a subscriber instead of the
/// text-shaped helpers.
fn read_product_child_output(fixture: &Path) -> (Vec<u8>, String) {
    let folder = tempfile::TempDir::new().expect("runtime folder");
    let (runtime, sink) = support::runtime_in(folder.path(), support::limits(1000));
    let agent = AgentId::generate();
    let fixture_argument = fixture.display().to_string();
    support::spawn_program(
        &runtime,
        agent,
        "cmd.exe",
        &["/d", "/c", "type", &fixture_argument],
    );
    let tap = Arc::new(MasterTap::default());
    let attachment = runtime
        .attach(
            agent,
            None,
            support::size(80, 24),
            Arc::clone(&tap) as Arc<dyn OutputSubscriber>,
        )
        .expect("attach tap");

    let exit_deadline = Instant::now() + CHILD_EXIT_LIMIT;
    while !sink.has_exited() && Instant::now() < exit_deadline {
        std::thread::sleep(CHILD_EXIT_POLL);
    }
    let ending = if sink.has_exited() {
        "the runtime reported the exit".to_owned()
    } else {
        "the runtime never reported an exit within the limit".to_owned()
    };
    std::thread::sleep(AFTER_EXIT_GRACE);

    let mut received = attachment.snapshot;
    received.extend_from_slice(&tap.collected.lock().expect("tap bytes").clone());
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
fn panic_with_evidence(label: &str, received: &[u8], ending: &str, handshake: &str) -> ! {
    let has_bell = received.contains(&BEL);
    let bell_offset = received.iter().position(|&byte| byte == BEL);
    let verdict = if has_bell { "PRESENT" } else { "ABSENT" };
    let text = String::from_utf8_lossy(received);
    let mut message = format!(
        "CONPTY_BEL_PROBE {label} verdict: BEL byte (0x07) is {verdict} in the master stream. \
         windows: {}. child {ending}. bytes read: {}. landmarks BEFORE: {}, AFTER: {}. \
         first 0x07 at offset: {bell_offset:?}. {handshake}. text: {text:?}. \
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
    let (received, ending, query_seen, reply_sent) =
        read_raw_child_output(&fixture.path().join(FIXTURE_NAME));
    let handshake = format!(
        "handshake: init cursor query ESC[6n seen: {query_seen}, reply ESC[1;1R sent: {reply_sent}"
    );
    panic_with_evidence("raw-with-handshake", &received, &ending, &handshake);
}

#[test]
fn probe_the_product_path_forwards_the_bell_byte() {
    let fixture = write_fixture();
    let (received, ending) = read_product_child_output(&fixture.path().join(FIXTURE_NAME));
    let handshake = "handshake: the product runtime owns the writer".to_owned();
    panic_with_evidence("product-path", &received, &ending, &handshake);
}

/// portable-pty prefers a `conpty.dll` next to the test binary over the
/// kernel32 exports, so a sideloaded newer console would answer the query
/// differently. No runner ships one and this experiment adds no binary
/// downloads, so with no DLL resolvable the test returns without failing
/// the job.
#[test]
fn probe_sideloaded_conpty_dll_forwards_the_bell_byte() {
    let Some(dll) = resolvable_conpty_dll() else {
        return;
    };
    let fixture = write_fixture();
    let (received, ending, query_seen, reply_sent) =
        read_raw_child_output(&fixture.path().join(FIXTURE_NAME));
    let label = format!("sideloaded dll {}", dll.display());
    let handshake = format!(
        "handshake: init cursor query ESC[6n seen: {query_seen}, reply ESC[1;1R sent: {reply_sent}"
    );
    panic_with_evidence(&label, &received, &ending, &handshake);
}

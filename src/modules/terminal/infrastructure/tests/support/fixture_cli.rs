// SPDX-License-Identifier: AGPL-3.0-or-later

//! A child program for the terminal tests. The first argument picks what it
//! does; every mode ends by itself after `WATCHDOG`, so a failed test leaves
//! no process behind.

use std::fmt::Write as _;
use std::io::{BufReader, Read, Write, stdin, stdout};
use std::process::{Command, Stdio, exit};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const WATCHDOG: Duration = Duration::from_secs(60);
const REPLY_TIMEOUT: Duration = Duration::from_secs(5);
const SPLIT_QUERY_GAP: Duration = Duration::from_millis(150);
const RELEASE_POLL: Duration = Duration::from_millis(10);
const SIZE_POLL: Duration = Duration::from_millis(50);
const USAGE_EXIT_CODE: i32 = 64;
const TIMEOUT_EXIT_CODE: i32 = 2;

fn main() {
    thread::spawn(|| {
        thread::sleep(WATCHDOG);
        exit(TIMEOUT_EXIT_CODE);
    });
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let argument = |index: usize| arguments.get(index).map_or("", String::as_str);
    match argument(0) {
        "exit" => exit(number(argument(1))),
        "exit-on-file" => exit_on_file(argument(1), number(argument(2))),
        "pause-resume" => pause_resume(number(argument(1))),
        "counter" => counter(number(argument(1)), number(argument(2))),
        "keys" => keys(),
        "lines" => lines(),
        "progress" => progress(),
        "altscreen" => altscreen(),
        "size" => size(),
        "query" => query(),
        "query-split" => query_split(),
        "sleep" => sleep_forever(),
        _ => exit(USAGE_EXIT_CODE),
    }
}

fn number<T: std::str::FromStr>(text: &str) -> T {
    text.parse().unwrap_or_else(|_| exit(USAGE_EXIT_CODE))
}

fn announce(text: &str) {
    let mut output = stdout().lock();
    // A closed terminal means the test is over.
    let _ = output
        .write_all(text.as_bytes())
        .and_then(|()| output.flush());
}

fn sleep_forever() -> ! {
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}

/// Raw mode makes every key reach this program unchanged, Ctrl+C included.
/// On Unix `stty` does it; on Windows `console-control` clears the line input
/// modes, which a pseudoconsole console starts with.
fn enter_raw_mode() {
    if cfg!(unix) {
        let _ = Command::new("stty")
            .args(["raw", "-echo"])
            .stdin(Stdio::inherit())
            .status();
    } else if let Err(text) = console_control::disable_line_input() {
        announce(&format!("RAW-ERROR:{text}\r\n"));
    }
}

/// Writes one line, stays silent until the file at `path` exists, and exits
/// with `code`. The test creates the file, so the silence lasts as long as
/// the test needs, whatever the scheduling.
fn exit_on_file(path: &str, code: i32) {
    announce("HELLO\n");
    while !std::path::Path::new(path).exists() {
        thread::sleep(RELEASE_POLL);
    }
    exit(code);
}

/// Writes, goes quiet, writes three lines close together, goes quiet again,
/// and exits with code 0.
fn pause_resume(pause_milliseconds: u64) {
    let pause = Duration::from_millis(pause_milliseconds);
    announce("ONE\n");
    thread::sleep(pause);
    for line in ["TWO\n", "THREE\n", "FOUR\n"] {
        announce(line);
        thread::sleep(Duration::from_millis(5));
    }
    thread::sleep(pause);
}

fn counter(count: u32, delay_milliseconds: u64) {
    for line in 0..count {
        announce(&format!("L{line:03}.\n"));
        thread::sleep(Duration::from_millis(delay_milliseconds));
    }
    announce("DONE\n");
}

fn keys() {
    enter_raw_mode();
    announce("READY\r\n");
    let mut byte = [0_u8; 1];
    while stdin().read(&mut byte).is_ok_and(|count| count == 1) {
        announce(&format!("KEY:{:02x}\r\n", byte[0]));
        if byte[0] == b'q' {
            break;
        }
    }
}

fn altscreen() {
    announce("\x1b[?1049h\x1b[2J\x1b[5;10HFULLSCREEN-MARKER");
    sleep_forever();
}

/// Prints one line per command byte and rings the bell on `b`. Tests drive
/// it through `write_input`, so one run serves several assertions.
fn lines() {
    enter_raw_mode();
    let mut byte = [0_u8; 1];
    while stdin().read(&mut byte).is_ok_and(|count| count == 1) {
        match byte[0] {
            // The bell byte goes out on its own write, so a read that ends
            // between it and the line break cannot lose it.
            b'b' => {
                announce("BELLED");
                announce("\x07");
                announce("\n");
            }
            other => announce(&format!("LINE:{other}\r\n")),
        }
    }
}

/// Redraws one row nine times, about 30 ms apart. Every redraw lands inside
/// the report throttle's one-second window, so the tests can hold the report
/// count without racing a monitor tick.
fn progress() {
    announce("PROGRESS:0%");
    for percent in 1..=9 {
        thread::sleep(Duration::from_millis(30));
        announce(&format!("\rPROGRESS:{percent}%"));
    }
    sleep_forever();
}

fn size() {
    let mut last = String::new();
    loop {
        let output = Command::new("stty")
            .arg("size")
            .stdin(Stdio::inherit())
            .output();
        if let Ok(output) = output {
            let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            if text != last {
                announce(&format!("SIZE:{text}\n"));
                last = text;
            }
        }
        thread::sleep(SIZE_POLL);
    }
}

/// Reads input bytes on a thread, so the caller can wait with a timeout.
fn input_bytes() -> mpsc::Receiver<u8> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        for byte in BufReader::new(stdin()).bytes().map_while(Result::ok) {
            if sender.send(byte).is_err() {
                break;
            }
        }
    });
    receiver
}

fn query() {
    enter_raw_mode();
    announce("READY\r\n");
    let input = input_bytes();
    announce("\x1b[6n");
    let mut reply = String::new();
    loop {
        match input.recv_timeout(REPLY_TIMEOUT) {
            Ok(byte) => {
                let _ = write!(reply, "{byte:02x}");
                if byte == b'R' {
                    break;
                }
            }
            Err(_) => exit(TIMEOUT_EXIT_CODE),
        }
    }
    announce(&format!("GOT:{reply}\r\n"));
}

/// Sends one cursor query in two writes and counts the answers that come back.
fn query_split() {
    enter_raw_mode();
    announce("READY\r\n");
    let input = input_bytes();
    announce("\x1b[6");
    thread::sleep(SPLIT_QUERY_GAP);
    announce("n");
    // A device status query after it is answered with ESC [ 0 n, in order,
    // so every cursor reply (ending in R) arrives before that n: the count
    // needs no time window.
    announce("\x1b[5n");
    let mut replies = 0;
    loop {
        match input.recv_timeout(REPLY_TIMEOUT) {
            Ok(b'R') => replies += 1,
            Ok(b'n') => break,
            Ok(_) => {}
            // No end marker: the count proves nothing, so the run fails.
            Err(_) => exit(TIMEOUT_EXIT_CODE),
        }
    }
    announce(&format!("REPLIES:{replies}\r\n"));
}

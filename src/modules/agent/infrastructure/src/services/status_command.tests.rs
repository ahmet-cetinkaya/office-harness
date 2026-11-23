// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use tempfile::TempDir;

use super::*;

const SHORT: Duration = Duration::from_millis(500);
const LONG: Duration = Duration::from_secs(20);

#[test]
fn a_program_that_cannot_start_gives_could_not_run() {
    let folder = TempDir::new().expect("folder");
    let result = run_status_command(
        &folder.path().join("no-such-program"),
        &[],
        folder.path(),
        &Environment::new(),
        SHORT,
    );
    assert!(
        matches!(result, StatusRunResult::CouldNotRun(_)),
        "{result:?}"
    );
}

#[cfg(windows)]
#[test]
fn a_timeout_ends_a_windows_command() {
    let folder = TempDir::new().expect("folder");
    let shell = std::env::var_os("ComSpec").map_or_else(|| PathBuf::from("cmd.exe"), PathBuf::from);
    let started = Instant::now();
    let result = run_status_command(
        &shell,
        &[
            "/d".to_owned(),
            "/c".to_owned(),
            "ping -n 30 127.0.0.1 >NUL".to_owned(),
        ],
        folder.path(),
        &std::env::vars().collect(),
        SHORT,
    );
    assert_eq!(result, StatusRunResult::TimedOut);
    assert!(started.elapsed() < Duration::from_secs(10));
}

/// A batch file, as an npm shim is, that starts a child and waits: the
/// deadline ends the batch file and the child it started (0019 L1).
#[cfg(windows)]
#[test]
fn a_timeout_ends_a_batch_file_and_its_child() {
    let folder = TempDir::new().expect("folder");
    let program = folder.path().join("cli.cmd");
    std::fs::write(&program, "@echo off\r\nping -n 30 127.0.0.1 >NUL\r\n").expect("batch file");
    let started = Instant::now();
    let result = run_status_command(
        &program,
        &["login".to_owned(), "status".to_owned()],
        folder.path(),
        &std::env::vars().collect(),
        SHORT,
    );
    assert_eq!(result, StatusRunResult::TimedOut);
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[cfg(unix)]
mod with_a_stand_in_program {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::thread;

    use super::*;

    const PROBE: &str = "--probe";
    const PROBE_ATTEMPTS: usize = 100;
    const END_DEADLINE: Duration = Duration::from_secs(5);
    const END_POLL: Duration = Duration::from_millis(10);

    /// A stand-in CLI. Another test thread that forks while the script is
    /// open for writing makes the first runs fail with "text file busy", so
    /// a harmless probe runs until the script starts.
    fn program(folder: &Path, script: &str) -> PathBuf {
        let file = folder.join("cli");
        fs::write(
            &file,
            format!("#!/bin/sh\n[ \"$1\" = \"{PROBE}\" ] && exit 0\n{script}\n"),
        )
        .expect("script");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("runnable");
        let starts = (0..PROBE_ATTEMPTS).any(|_| {
            let started = Command::new(&file).arg(PROBE).status().is_ok();
            if !started {
                thread::sleep(END_POLL);
            }
            started
        });
        assert!(starts, "the stand-in CLI never started");
        file
    }

    fn environment(pairs: &[(&str, &str)]) -> Environment {
        pairs
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect()
    }

    fn run(script: &str, timeout: Duration) -> (TempDir, StatusRunResult) {
        let folder = TempDir::new().expect("folder");
        let program = program(folder.path(), script);
        let result = run_status_command(
            &program,
            &["auth".to_owned(), "status".to_owned()],
            folder.path(),
            &environment(&[("PATH", "/usr/bin:/bin"), ("ONLY", "this")]),
            timeout,
        );
        (folder, result)
    }

    fn has_ended(pid: i32) -> bool {
        let deadline = Instant::now() + END_DEADLINE;
        while nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), None).is_ok() {
            let is_zombie = fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|stat| {
                stat.rsplit_once(") ")
                    .is_some_and(|(_, rest)| rest.starts_with('Z'))
            });
            if is_zombie {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(END_POLL);
        }
        true
    }

    #[test]
    fn the_exit_code_and_both_streams_come_back_standard_output_first() {
        let (_folder, result) = run("printf '{}'; echo 'Not logged in' >&2; exit 1", LONG);
        assert_eq!(
            result,
            StatusRunResult::Ended {
                exit_code: Some(1),
                output: "{}\nNot logged in\n".to_owned(),
            }
        );
    }

    #[test]
    fn the_command_gets_its_arguments_the_folder_and_exactly_the_environment() {
        let (folder, result) = run("echo \"$1 $2|$ONLY|${HOME:-none}|$(pwd)\"", LONG);
        let expected = format!(
            "auth status|this|none|{}\n",
            folder.path().canonicalize().expect("folder").display()
        );
        assert_eq!(
            result,
            StatusRunResult::Ended {
                exit_code: Some(0),
                output: expected,
            }
        );
    }

    #[test]
    fn the_input_is_empty() {
        let (_folder, result) = run("read line; echo \"read:$?\"", LONG);
        assert_eq!(
            result,
            StatusRunResult::Ended {
                exit_code: Some(0),
                output: "read:1\n".to_owned(),
            }
        );
    }

    #[test]
    fn at_most_the_cap_is_kept() {
        let (_folder, result) = run("head -c 1048576 /dev/zero | tr '\\0' 'x'; exit 1", LONG);
        let StatusRunResult::Ended { output, exit_code } = result else {
            panic!("expected an end");
        };
        assert_eq!(exit_code, Some(1));
        assert!(output.len() <= STATUS_OUTPUT_CAP_BYTES, "{}", output.len());
    }

    #[test]
    fn a_timeout_ends_the_command_and_its_children() {
        let folder = TempDir::new().expect("folder");
        let pid_file = folder.path().join("child.pid");
        let script = format!("sleep 30 & echo $! > '{}'; wait", pid_file.display());
        let program = program(folder.path(), &script);
        let started = Instant::now();
        let result = run_status_command(
            &program,
            &[],
            folder.path(),
            &environment(&[("PATH", "/usr/bin:/bin")]),
            SHORT,
        );
        assert_eq!(result, StatusRunResult::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(5));
        let child: i32 = fs::read_to_string(&pid_file)
            .expect("pid")
            .trim()
            .parse()
            .expect("number");
        assert!(has_ended(child), "the child {child} still runs");
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn the_child_environment_drops_git_and_askpass_variables() {
    let login: Environment = [
        ("PATH", "/bin"),
        ("GIT_DIR", "/elsewhere"),
        ("GIT_WORK_TREE", "/elsewhere"),
        ("git_askpass", "/x"),
        ("SSH_ASKPASS", "/x"),
        ("LC_ALL", "de_DE"),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_owned(), value.to_owned()))
    .collect();
    let environment = child_environment(&login);
    assert_eq!(environment.get("PATH").map(String::as_str), Some("/bin"));
    for dropped in ["GIT_DIR", "GIT_WORK_TREE", "git_askpass", "SSH_ASKPASS"] {
        assert!(!environment.contains_key(dropped), "{dropped}");
    }
    assert_eq!(environment.get("LC_ALL").map(String::as_str), Some("C"));
    assert_eq!(
        environment.get("GIT_TERMINAL_PROMPT").map(String::as_str),
        Some("0")
    );
    assert_eq!(
        environment.get("GCM_INTERACTIVE").map(String::as_str),
        Some("never")
    );
}

#[test]
fn versions_of_every_platform_parse() {
    assert_eq!(parse_version("git version 2.56.0\n"), Some((2, 56)));
    assert_eq!(parse_version("git version 2.45.1.windows.1"), Some((2, 45)));
    assert_eq!(
        parse_version("git version 2.39.3 (Apple Git-145)"),
        Some((2, 39))
    );
    assert_eq!(parse_version("hello"), None);
    assert!(parse_version("git version 2.21.0").is_some_and(|v| v < MINIMUM_VERSION));
}

#[test]
fn the_reason_loses_url_credentials_and_is_cut() {
    let reason = clean_reason(
        "fatal: unable to access 'https://bob:secret@example.com/r.git/':\n  denied ssh://git@host/x",
    );
    assert_eq!(
        reason,
        "fatal: unable to access 'https://example.com/r.git/': denied ssh://host/x"
    );
    assert_eq!(
        clean_reason(&"x".repeat(900)).chars().count(),
        REASON_MAX_CHARS
    );
}

#[test]
fn the_reason_loses_control_and_bidirectional_format_characters() {
    assert_eq!(
        clean_reason("fatal: \u{1b}[31mbad\u{7} name \u{202E}txt.exe\u{2066}"),
        "fatal: [31mbad name txt.exe"
    );
}

#[cfg(unix)]
mod with_a_stand_in_program {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use std::thread;

    use tempfile::TempDir;

    use super::*;

    const PROBE: &str = "--probe";
    const PROBE_ATTEMPTS: usize = 100;

    /// A stand-in git. Another test thread that forks while the script is
    /// open for writing makes the first runs fail with "text file busy", so
    /// the helper runs a harmless probe until the script starts.
    fn program(folder: &Path, script: &str) -> PathBuf {
        let file = folder.join("git");
        fs::write(
            &file,
            format!("#!/bin/sh\n[ \"$1\" = \"{PROBE}\" ] && exit 0\n{script}\n"),
        )
        .expect("script");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("runnable");
        let starts = (0..PROBE_ATTEMPTS).any(|_| {
            let started = std::process::Command::new(&file)
                .arg(PROBE)
                .status()
                .is_ok();
            if !started {
                thread::sleep(Duration::from_millis(10));
            }
            started
        });
        assert!(starts, "the stand-in git never started");
        file
    }

    const END_DEADLINE: Duration = Duration::from_secs(5);
    const END_POLL: Duration = Duration::from_millis(10);

    /// True once `pid` is gone or a zombie, polled up to a deadline: a killed
    /// child whose parent already exited waits for the system to reap it,
    /// and until then a signal check still finds it.
    fn has_ended(pid: i32) -> bool {
        let deadline = Instant::now() + END_DEADLINE;
        while !is_gone_or_zombie(pid) {
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(END_POLL);
        }
        true
    }

    /// Linux shows a zombie's state in `/proc`; elsewhere a zombie counts as
    /// running until it is reaped, which the polling waits for.
    fn is_gone_or_zombie(pid: i32) -> bool {
        const ZOMBIE_STATE: char = 'Z';
        if nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), None).is_err() {
            return true;
        }
        fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|stat| {
            stat.rsplit_once(") ")
                .is_some_and(|(_, fields)| fields.starts_with(ZOMBIE_STATE))
        })
    }

    fn runner_for(folder: &Path, script: &str, environment: &Environment) -> GitRunner {
        GitRunner::new(program(folder, script), environment)
    }

    fn environment() -> Environment {
        [
            ("PATH", "/usr/bin:/bin"),
            ("GIT_DIR", "/elsewhere"),
            ("GIT_WORK_TREE", "/w"),
        ]
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect()
    }

    #[test]
    fn the_server_environment_cannot_point_git_elsewhere() {
        let temp = TempDir::new().expect("temp");
        let runner = runner_for(temp.path(), "env", &environment());
        let output = runner.read(temp.path(), &["x"]).expect("ran");
        assert!(!output.contains("GIT_DIR="), "{output}");
        assert!(!output.contains("GIT_WORK_TREE="), "{output}");
        assert!(output.contains("GIT_OPTIONAL_LOCKS=0"), "{output}");
        assert!(output.contains("GIT_NO_LAZY_FETCH=1"), "{output}");
        assert!(output.contains("GIT_ALLOW_PROTOCOL=none/"), "{output}");
        let output = runner.write(temp.path(), &["x"]).expect("ran");
        assert!(!output.contains("GIT_OPTIONAL_LOCKS"), "{output}");
    }

    #[test]
    fn hooks_and_the_folder_come_first_and_arguments_stay_a_list() {
        let temp = TempDir::new().expect("temp");
        let runner = runner_for(temp.path(), r#"printf '%s\n' "$@""#, &environment());
        let output = runner
            .read(temp.path(), &["a b", "; rm -rf /"])
            .expect("ran");
        let arguments: Vec<&str> = output.lines().collect();
        let folder = temp.path().display().to_string();
        let mut expected = vec!["-c", NO_HOOKS_PATH];
        for switch in DISABLED_HOOK_EVENTS {
            expected.extend(["-c", switch]);
        }
        expected.extend([
            "-c",
            "core.fsmonitor=false",
            "-C",
            folder.as_str(),
            "a b",
            "; rm -rf /",
        ]);
        assert_eq!(arguments, expected);
    }

    #[test]
    fn more_than_one_mebibyte_of_output_fails() {
        let temp = TempDir::new().expect("temp");
        let runner = runner_for(temp.path(), "head -c 1100000 /dev/zero", &environment());
        assert_eq!(
            runner.read(temp.path(), &["x"]),
            Err(GitFailure::OutputTooLarge)
        );
    }

    #[test]
    fn a_failure_keeps_the_cleaned_reason() {
        let temp = TempDir::new().expect("temp");
        let runner = runner_for(
            temp.path(),
            "echo 'fatal: https://u:p@h/x' >&2; exit 128",
            &environment(),
        );
        assert_eq!(
            runner.read(temp.path(), &["x"]),
            Err(GitFailure::Exit("fatal: https://h/x".to_owned()))
        );
    }

    #[test]
    fn a_timeout_ends_git_and_its_children() {
        let temp = TempDir::new().expect("temp");
        let pid_file = temp.path().join("child.pid");
        let script = format!("sleep 30 & echo $! > '{}'; wait", pid_file.display());
        let runner = runner_for(temp.path(), &script, &environment())
            .with_timeouts(Duration::from_millis(500), Duration::from_millis(500));
        let started = Instant::now();
        assert_eq!(runner.read(temp.path(), &["x"]), Err(GitFailure::Timeout));
        assert!(started.elapsed() < Duration::from_secs(5));
        let child: i32 = fs::read_to_string(&pid_file)
            .expect("pid")
            .trim()
            .parse()
            .expect("number");
        assert!(has_ended(child), "the child {child} still runs");
    }

    #[test]
    fn a_child_that_keeps_the_pipes_after_git_exits_is_ended_and_the_output_kept() {
        let temp = TempDir::new().expect("temp");
        let pid_file = temp.path().join("child.pid");
        let script = format!("sleep 30 & echo $! > '{}'; echo out", pid_file.display());
        let runner = runner_for(temp.path(), &script, &environment())
            .with_timeouts(Duration::from_secs(20), Duration::from_secs(20));
        let started = Instant::now();
        assert_eq!(runner.read(temp.path(), &["x"]), Ok("out\n".to_owned()));
        assert!(started.elapsed() < Duration::from_secs(5));
        let child: i32 = fs::read_to_string(&pid_file)
            .expect("pid")
            .trim()
            .parse()
            .expect("number");
        assert!(has_ended(child), "the child {child} still runs");
    }

    /// `setsid` leaves git's group, so ending the group cannot close the
    /// pipe; the success still stands, because only stdout must end.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_helper_outside_the_group_that_keeps_stderr_does_not_turn_a_success_into_a_timeout() {
        let temp = TempDir::new().expect("temp");
        let pid_file = temp.path().join("helper.pid");
        let script = format!(
            "setsid sleep 30 >/dev/null </dev/null & echo $! > '{}'; echo out",
            pid_file.display()
        );
        let runner = runner_for(temp.path(), &script, &environment())
            .with_timeouts(Duration::from_secs(20), Duration::from_secs(20));
        let started = Instant::now();
        let answer = runner.read(temp.path(), &["x"]);
        let helper: i32 = fs::read_to_string(&pid_file)
            .expect("pid")
            .trim()
            .parse()
            .expect("number");
        let _ = nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(helper),
            nix::sys::signal::Signal::SIGKILL,
        );
        assert_eq!(answer, Ok("out\n".to_owned()));
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn an_old_git_is_not_supported() {
        let temp = TempDir::new().expect("temp");
        let old = runner_for(temp.path(), "echo 'git version 2.21.4'", &environment());
        assert!(!old.is_supported());
        let new = runner_for(temp.path(), "echo 'git version 2.22.0'", &environment());
        assert!(new.is_supported());
    }
}

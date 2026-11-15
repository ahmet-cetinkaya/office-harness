// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn environment(entries: &[(&str, &str)]) -> Environment {
    entries
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

#[test]
fn removes_the_server_own_variables() {
    let server = environment(&[
        ("APPDIR", "/appimage/mount"),
        ("APPIMAGE", "/x/office.AppImage"),
        ("APPIMAGE_SIGNATURE", "x"),
        ("LD_LIBRARY_PATH", "/appimage/mount/usr/lib"),
        ("PWD", "/somewhere"),
        ("OLDPWD", "/before"),
        ("SHLVL", "2"),
        ("_", "/usr/bin/env"),
        ("HOME", "/user/home"),
    ]);
    assert_eq!(
        without_server_variables(&server),
        environment(&[("HOME", "/user/home")])
    );
}

#[cfg(unix)]
#[test]
fn removes_the_path_entries_inside_the_app_image() {
    let server = environment(&[
        ("APPDIR", "/appimage/mount"),
        ("PATH", "/appimage/mount/usr/bin:/usr/bin:/bin"),
    ]);
    assert_eq!(
        without_server_variables(&server)
            .get("PATH")
            .map(String::as_str),
        Some("/usr/bin:/bin")
    );
}

#[cfg(unix)]
mod unix_shell {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::{Mutex, MutexGuard, PoisonError};
    use std::time::Instant;

    use tempfile::TempDir;

    use super::*;

    static SHELL_TESTS: Mutex<()> = Mutex::new(());

    /// A script that was just written cannot be started while another thread
    /// holds a forked copy of its still-open file ("text file busy"), so the
    /// tests that write and start a script take turns.
    fn one_at_a_time() -> MutexGuard<'static, ()> {
        SHELL_TESTS.lock().unwrap_or_else(PoisonError::into_inner)
    }

    const GENEROUS_TIMEOUT: Duration = Duration::from_secs(5);
    const SHORT_TIMEOUT: Duration = Duration::from_millis(400);

    /// A stand-in login shell. It ends the way a real one does: with the
    /// markers around the NUL-separated variables. When it is asked to be
    /// interactive (`-i`) and `hang_when_interactive` is set, it never answers.
    fn stand_in_shell(folder: &TempDir, variables: &str, hang_when_interactive: bool) -> PathBuf {
        let hang = if hang_when_interactive {
            "for argument in \"$@\"; do\n  if [ \"$argument\" = \"-i\" ]; then while :; do :; done; fi\ndone\n"
        } else {
            ""
        };
        let script = format!(
            "#!/bin/sh\n{hang}echo 'noise from a start file'\nprintf '%s' '{MARKER}'\nprintf '{variables}'\nprintf '%s' '{MARKER}'\n",
            MARKER = login_shell::MARKER_FOR_TESTS,
        );
        let path = folder.path().join("stand-in-shell");
        fs::write(&path, script).expect("write script");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("make executable");
        path
    }

    #[test]
    fn returns_the_login_shell_path_from_an_empty_environment() {
        let _turn = one_at_a_time();
        let folder = TempDir::new().expect("temp folder");
        let shell = stand_in_shell(&folder, "PATH=/login/bin\\0HOME=/user/home\\0", false);
        let service = LoginEnvironment::with_shells(Some(shell), None, GENEROUS_TIMEOUT);

        let resolved = service.resolve(&Environment::new());

        assert_eq!(
            resolved.variables.get("PATH").map(String::as_str),
            Some("/login/bin")
        );
        assert_eq!(resolved.warning, None);
    }

    #[test]
    fn falls_back_with_a_warning_when_the_shell_start_files_hang() {
        let _turn = one_at_a_time();
        let folder = TempDir::new().expect("temp folder");
        let shell = stand_in_shell(&folder, "PATH=/fallback/bin\\0", true);
        let service =
            LoginEnvironment::with_shells(Some(shell.clone()), Some(shell), SHORT_TIMEOUT);

        let started = Instant::now();
        let resolved = service.resolve(&Environment::new());

        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(
            resolved.variables.get("PATH").map(String::as_str),
            Some("/fallback/bin")
        );
        assert!(resolved.warning.is_some());
    }

    #[test]
    fn uses_the_server_environment_with_a_warning_when_no_shell_answers() {
        let service = LoginEnvironment::with_shells(None, None, SHORT_TIMEOUT);
        let server = environment(&[("PATH", "/server/bin"), ("APPDIR", "/appimage/mount")]);

        let resolved = service.resolve(&server);

        assert_eq!(resolved.variables, environment(&[("PATH", "/server/bin")]));
        assert!(resolved.warning.is_some());
    }

    #[test]
    fn removes_the_app_image_variables_from_the_shell_answer() {
        let _turn = one_at_a_time();
        let folder = TempDir::new().expect("temp folder");
        let shell = stand_in_shell(
            &folder,
            "APPDIR=/appimage/mount\\0LD_LIBRARY_PATH=/appimage/mount/lib\\0PATH=/usr/bin\\0",
            false,
        );
        let service = LoginEnvironment::with_shells(Some(shell), None, GENEROUS_TIMEOUT);

        let resolved = service.resolve(&Environment::new());

        assert_eq!(resolved.variables, environment(&[("PATH", "/usr/bin")]));
    }
}

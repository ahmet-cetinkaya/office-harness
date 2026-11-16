// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! A login environment stand-in and fake agent CLIs, so a test can start
//! agents through the whole server without a real CLI or the user's shell.
//! On Unix the stand-in is a login shell script and the fake CLIs are `sh`
//! scripts. On Windows the environment is fixed (the registry would give the
//! user's real `PATH` and profile folders) and the fake CLIs are `.cmd` files.

use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use agent_infrastructure::services::{AttributionTiming, Environment, LoginEnvironment};
use office_harnessd::bootstrap::LoginSetup;
use tempfile::TempDir;

#[cfg(unix)]
const MARKER: &str = "__OFFICE_HARNESS_ENVIRONMENT__";
#[cfg(unix)]
const SHELL_TIMEOUT: Duration = Duration::from_secs(5);
/// What a script needs for `sleep` and `cat`, since the login PATH of the
/// stand-in holds only the tool folder.
#[cfg(unix)]
const SCRIPT_PATH: &str = "PATH=/usr/bin:/bin";
#[cfg(windows)]
const SYSTEM_ROOT_VARIABLE: &str = "SystemRoot";
#[cfg(windows)]
const SYSTEM_FOLDER: &str = "System32";
/// How long a fake CLI stays alive after it wrote what the test looks for.
const KEEP_ALIVE_SECONDS: u32 = 30;
/// How the fake CLIs' folders of the fake home are named under it: the fake
/// home and the fake XDG folders keep every test off the real folders of the
/// machine's user.
const HOME_FOLDER: &str = "home";
const XDG_CONFIG_FOLDER: &str = "xdg-config";
const XDG_DATA_FOLDER: &str = "xdg-data";
const XDG_STATE_FOLDER: &str = "xdg-state";
const XDG_CACHE_FOLDER: &str = "xdg-cache";
/// A window that is short for a test that waits for it to close.
pub const SHORT_ATTRIBUTION: Duration = Duration::from_millis(600);
const ATTRIBUTION_POLL: Duration = Duration::from_millis(20);
const ATTRIBUTION_TIMEOUT: Duration = Duration::from_secs(3);

/// A folder of fake CLIs that only the stand-in login environment's `PATH`
/// knows.
pub struct FakeClis {
    _folder: TempDir,
    tool_folder: PathBuf,
    /// How long the stand-in login shell takes to answer, in seconds.
    #[cfg(unix)]
    login_delay_seconds: u32,
    /// Folders after the tool folder on the fixed `PATH`, such as the one of
    /// `node.exe`.
    #[cfg(windows)]
    extra_path_folders: Vec<PathBuf>,
    attribution: AttributionTiming,
}

impl FakeClis {
    #[must_use]
    pub fn new() -> Self {
        let folder = TempDir::new().expect("folder");
        let tool_folder = folder.path().join("tools");
        fs::create_dir_all(&tool_folder).expect("tools folder");
        for name in [
            HOME_FOLDER,
            XDG_CONFIG_FOLDER,
            XDG_DATA_FOLDER,
            XDG_STATE_FOLDER,
            XDG_CACHE_FOLDER,
        ] {
            fs::create_dir_all(folder.path().join(name)).expect("fake folder");
        }
        Self {
            _folder: folder,
            tool_folder,
            #[cfg(unix)]
            login_delay_seconds: 0,
            #[cfg(windows)]
            extra_path_folders: Vec::new(),
            attribution: AttributionTiming {
                poll_interval: ATTRIBUTION_POLL,
                timeout: ATTRIBUTION_TIMEOUT,
                settle: ATTRIBUTION_POLL,
            },
        }
    }

    /// The folder that holds the fake CLIs, the first one on the login `PATH`.
    #[must_use]
    pub fn tool_folder(&self) -> &Path {
        &self.tool_folder
    }

    /// The fake home folder that every fake CLI sees as `HOME`.
    #[must_use]
    pub fn home_folder(&self) -> PathBuf {
        self.tool_folder.with_file_name(HOME_FOLDER)
    }

    /// The fake XDG folders, which hold the fake configuration of the CLIs
    /// that follow XDG. On Windows the first two are also `APPDATA` and
    /// `LOCALAPPDATA`.
    #[must_use]
    pub fn xdg_folders(&self) -> [PathBuf; 4] {
        [
            XDG_CONFIG_FOLDER,
            XDG_DATA_FOLDER,
            XDG_STATE_FOLDER,
            XDG_CACHE_FOLDER,
        ]
        .map(|name| self.tool_folder.with_file_name(name))
    }

    /// How long to look for the session id of a CLI that does not take one.
    #[must_use]
    pub fn with_attribution_timeout(mut self, timeout: Duration) -> Self {
        self.attribution.timeout = timeout;
        self
    }

    /// Puts `folder` on the fixed login `PATH`, after the tool folder.
    #[cfg(windows)]
    #[must_use]
    pub fn with_path_folder(mut self, folder: PathBuf) -> Self {
        self.extra_path_folders.push(folder);
        self
    }

    /// A login shell that answers only after `seconds`, so a start is slow.
    #[cfg(unix)]
    #[must_use]
    pub fn with_slow_login(mut self, seconds: u32) -> Self {
        self.login_delay_seconds = seconds;
        self
    }

    /// Adds an executable `name` that runs `body` with `sh`.
    #[cfg(unix)]
    pub fn add(&self, name: &str, body: &str) {
        let path = self.tool_folder.join(name);
        fs::write(&path, format!("#!/bin/sh\n{SCRIPT_PATH}\n{body}\n")).expect("write script");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("make executable");
    }

    /// Adds the batch file `name.cmd` that runs `body` with `cmd.exe`. The
    /// lookup finds it by `name`, as it finds an npm shim.
    #[cfg(windows)]
    pub fn add(&self, name: &str, body: &str) {
        let path = self.tool_folder.join(format!("{name}.cmd"));
        let lines: Vec<&str> = std::iter::once("@echo off").chain(body.lines()).collect();
        fs::write(&path, lines.join("\r\n")).expect("write script");
    }

    /// A server environment that holds only an `AppImage` variable, and a
    /// login environment whose `PATH` is the tool folder alone.
    #[must_use]
    pub fn login_setup(&self) -> LoginSetup {
        LoginSetup {
            login_environment: Arc::new(self.login_environment()),
            server_environment: Environment::from([(
                "APPDIR".to_owned(),
                "/appimage/mount".to_owned(),
            )]),
            attribution: self.attribution,
        }
    }

    #[cfg(unix)]
    fn login_environment(&self) -> LoginEnvironment {
        LoginEnvironment::with_shells(Some(self.stand_in_login_shell()), None, SHELL_TIMEOUT)
    }

    /// The tool folder and the system folder (for `ping` and `cmd.exe`) are
    /// the whole `PATH`. The profile and temporary folders point into the
    /// fake home; `ComSpec` and `windir` are what `cmd.exe` expects to find.
    #[cfg(windows)]
    fn login_environment(&self) -> LoginEnvironment {
        let system_root = std::env::var(SYSTEM_ROOT_VARIABLE).expect("the system root");
        let path = std::env::join_paths(
            [
                self.tool_folder.clone(),
                Path::new(&system_root).join(SYSTEM_FOLDER),
            ]
            .into_iter()
            .chain(self.extra_path_folders.iter().cloned()),
        )
        .expect("path entries");
        let home = self.home_folder();
        let [config, data, state, cache] = self.xdg_folders();
        let text = |path: &Path| path.display().to_string();
        LoginEnvironment::fixed(Environment::from([
            ("PATH".to_owned(), text(Path::new(&path))),
            (
                "ComSpec".to_owned(),
                text(&Path::new(&system_root).join(SYSTEM_FOLDER).join("cmd.exe")),
            ),
            ("windir".to_owned(), system_root.clone()),
            ("TEMP".to_owned(), text(&home)),
            ("TMP".to_owned(), text(&home)),
            (SYSTEM_ROOT_VARIABLE.to_owned(), system_root),
            ("USERPROFILE".to_owned(), text(&home)),
            ("HOME".to_owned(), text(&home)),
            ("APPDATA".to_owned(), text(&config)),
            ("LOCALAPPDATA".to_owned(), text(&data)),
            ("XDG_CONFIG_HOME".to_owned(), text(&config)),
            ("XDG_DATA_HOME".to_owned(), text(&data)),
            ("XDG_STATE_HOME".to_owned(), text(&state)),
            ("XDG_CACHE_HOME".to_owned(), text(&cache)),
        ]))
    }

    #[cfg(unix)]
    fn stand_in_login_shell(&self) -> PathBuf {
        let shell = self.tool_folder.with_file_name("login-shell");
        let [config, data, state, cache] = self.xdg_folders();
        let script = format!(
            "#!/bin/sh\n{SCRIPT_PATH}\nsleep {}\nprintf '%s' '{MARKER}'\nprintf 'PATH={}\\0APPDIR=/appimage/mount\\0HOME={}\\0XDG_CONFIG_HOME={}\\0XDG_DATA_HOME={}\\0XDG_STATE_HOME={}\\0XDG_CACHE_HOME={}\\0'\nprintf '%s' '{MARKER}'\n",
            self.login_delay_seconds,
            self.tool_folder.display(),
            self.home_folder().display(),
            config.display(),
            data.display(),
            state.display(),
            cache.display(),
        );
        write_executable(&shell, &script);
        shell
    }
}

#[cfg(unix)]
fn write_executable(path: &Path, script: &str) {
    fs::write(path, script).expect("write script");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("make executable");
}

/// A script body that prints `text` and then keeps the CLI alive for the
/// test. On Windows it reads a line first, because the console only echoes
/// typed input while a program reads it; on Unix the terminal echoes it
/// anyway. The test proves the typed startup prompt by that echo.
#[must_use]
pub fn print_then_wait(text: &str) -> String {
    if cfg!(windows) {
        format!(
            "echo {text}\nset /p TYPED=\nping -n {} 127.0.0.1 >nul",
            KEEP_ALIVE_SECONDS + 1
        )
    } else {
        format!("echo {text}; sleep {KEEP_ALIVE_SECONDS}")
    }
}

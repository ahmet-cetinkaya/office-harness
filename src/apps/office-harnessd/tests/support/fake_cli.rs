// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! A login shell stand-in and fake agent CLIs, so a test can start agents
//! through the whole server without a real CLI or the user's shell.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use agent_infrastructure::services::{Environment, LoginEnvironment};
use office_harnessd::bootstrap::LoginSetup;
use tempfile::TempDir;

const MARKER: &str = "__OFFICE_HARNESS_ENVIRONMENT__";
const SHELL_TIMEOUT: Duration = Duration::from_secs(5);
/// What a script needs for `sleep` and `cat`, since the login PATH of the
/// stand-in holds only the tool folder.
const SCRIPT_PATH: &str = "PATH=/usr/bin:/bin";

/// A folder of fake CLIs that only the stand-in login shell's `PATH` knows.
pub struct FakeClis {
    _folder: TempDir,
    tool_folder: PathBuf,
    /// How long the stand-in login shell takes to answer, in seconds.
    login_delay_seconds: u32,
}

impl FakeClis {
    #[must_use]
    pub fn new() -> Self {
        let folder = TempDir::new().expect("folder");
        let tool_folder = folder.path().join("tools");
        fs::create_dir_all(&tool_folder).expect("tools folder");
        Self {
            _folder: folder,
            tool_folder,
            login_delay_seconds: 0,
        }
    }

    /// A login shell that answers only after `seconds`, so a start is slow.
    #[must_use]
    pub fn with_slow_login(mut self, seconds: u32) -> Self {
        self.login_delay_seconds = seconds;
        self
    }

    /// Adds an executable `name` that runs `body` with `sh`.
    pub fn add(&self, name: &str, body: &str) {
        let path = self.tool_folder.join(name);
        fs::write(&path, format!("#!/bin/sh\n{SCRIPT_PATH}\n{body}\n")).expect("write script");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("make executable");
    }

    /// A server environment that holds only an `AppImage` variable, and a
    /// login shell whose `PATH` is the tool folder alone.
    #[must_use]
    pub fn login_setup(&self) -> LoginSetup {
        LoginSetup {
            login_environment: Arc::new(LoginEnvironment::with_shells(
                Some(self.stand_in_login_shell()),
                None,
                SHELL_TIMEOUT,
            )),
            server_environment: Environment::from([(
                "APPDIR".to_owned(),
                "/appimage/mount".to_owned(),
            )]),
        }
    }

    fn stand_in_login_shell(&self) -> PathBuf {
        let shell = self.tool_folder.with_file_name("login-shell");
        let script = format!(
            "#!/bin/sh\n{SCRIPT_PATH}\nsleep {}\nprintf '%s' '{MARKER}'\nprintf 'PATH={}\\0APPDIR=/appimage/mount\\0'\nprintf '%s' '{MARKER}'\n",
            self.login_delay_seconds,
            self.tool_folder.display()
        );
        write_executable(&shell, &script);
        shell
    }
}

fn write_executable(path: &Path, script: &str) {
    fs::write(path, script).expect("write script");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("make executable");
}

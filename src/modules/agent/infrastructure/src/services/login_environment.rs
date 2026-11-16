// SPDX-License-Identifier: AGPL-3.0-or-later

//! The environment an agent CLI starts with (FR-020): the user's login
//! environment, without the server's own variables. A server that systemd or
//! launchd starts has a minimal `PATH`, so the tools that the user's shell
//! sets up (npm, nvm, Homebrew) would not be found without this.

use std::collections::BTreeMap;
#[cfg(unix)]
use std::path::PathBuf;
#[cfg(unix)]
use std::time::Duration;

#[cfg(unix)]
use crate::services::login_shell;

pub type Environment = BTreeMap<String, String>;

/// How long a login shell may take before the service gives up on it.
#[cfg(unix)]
const SHELL_TIMEOUT: Duration = Duration::from_secs(5);
#[cfg(unix)]
const INTERACTIVE_LOGIN_ARGUMENTS: [&str; 2] = ["-l", "-i"];
#[cfg(unix)]
const LOGIN_ARGUMENTS: [&str; 1] = ["-l"];
#[cfg(unix)]
const WARNING_ACCOUNT_SHELL: &str = "The login shell did not give its environment. \
The environment comes from the account shell without interactive mode.";
#[cfg(unix)]
const WARNING_SERVER_ENVIRONMENT: &str =
    "No login shell gave its environment. The agent starts with the server's own environment.";
#[cfg(windows)]
const WARNING_REGISTRY: &str = "The registry environment could not be read. \
The agent starts with the server's own environment.";

/// Variables that belong to the server's own packaging, not to the user. An
/// `AppImage` sets them for its own libraries, and a child that inherits them
/// loads the wrong libraries.
const SERVER_OWN_VARIABLES: [&str; 13] = [
    "APPDIR",
    "APPIMAGE",
    "ARGV0",
    "OWD",
    "LD_LIBRARY_PATH",
    "PYTHONHOME",
    "PERLLIB",
    "GSETTINGS_SCHEMA_DIR",
    "GIO_EXTRA_MODULES",
    "GDK_PIXBUF_MODULE_FILE",
    "GTK_PATH",
    "GTK_EXE_PREFIX",
    "QT_PLUGIN_PATH",
];
const SERVER_OWN_PREFIX: &str = "APPIMAGE_";
/// The shell that ran to read the environment, and the server's own shell
/// session, set these for themselves. They describe that run, not the user.
const SHELL_SESSION_VARIABLES: [&str; 4] = ["PWD", "OLDPWD", "SHLVL", "_"];

/// The environment to give a child, and a warning when it is not the one the
/// user's login shell would give.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedEnvironment {
    pub variables: Environment,
    pub warning: Option<String>,
}

/// Resolves the login environment at each spawn.
pub struct LoginEnvironment {
    #[cfg(unix)]
    shells: ShellChoice,
    /// Set by `fixed` only: the environment to give, whatever the OS holds.
    fixed: Option<Environment>,
}

#[cfg(unix)]
struct ShellChoice {
    interactive: Option<PathBuf>,
    account: Option<PathBuf>,
    account_variables: Vec<(String, String)>,
    timeout: Duration,
}

impl LoginEnvironment {
    /// Picks the shells from the server's environment and the account record.
    /// The account record is the fallback, because `$SHELL` can be empty.
    #[cfg(unix)]
    #[must_use]
    pub fn detect(server_environment: &Environment) -> Self {
        let account = login_shell::current_account();
        let from_variable = server_environment
            .get("SHELL")
            .filter(|path| !path.is_empty())
            .map(PathBuf::from);
        let account_shell = account.as_ref().map(|account| account.shell.clone());
        Self {
            shells: ShellChoice {
                interactive: from_variable.or_else(|| account_shell.clone()),
                account: account_shell,
                account_variables: account
                    .as_ref()
                    .map_or_else(Default::default, login_shell::Account::variables),
                timeout: SHELL_TIMEOUT,
            },
            fixed: None,
        }
    }

    /// Uses the given shells and timeout; for tests with a stand-in shell.
    #[cfg(unix)]
    #[must_use]
    pub fn with_shells(
        interactive: Option<PathBuf>,
        account: Option<PathBuf>,
        timeout: Duration,
    ) -> Self {
        Self {
            shells: ShellChoice {
                interactive,
                account,
                account_variables: Vec::new(),
                timeout,
            },
            fixed: None,
        }
    }

    #[cfg(windows)]
    #[must_use]
    pub fn detect(_server_environment: &Environment) -> Self {
        Self { fixed: None }
    }

    /// Always resolves to `environment`, on any OS; for tests. The Windows
    /// registry holds the user's real `PATH` and profile folders, which a
    /// test cannot replace.
    #[must_use]
    pub fn fixed(environment: Environment) -> Self {
        Self {
            #[cfg(unix)]
            shells: ShellChoice {
                interactive: None,
                account: None,
                account_variables: Vec::new(),
                timeout: SHELL_TIMEOUT,
            },
            fixed: Some(environment),
        }
    }

    /// The fixed environment when there is one, otherwise the one the OS
    /// gives.
    #[must_use]
    pub fn resolve(&self, server_environment: &Environment) -> ResolvedEnvironment {
        match &self.fixed {
            Some(environment) => resolved(environment, None),
            None => self.resolve_from_system(server_environment),
        }
    }

    /// Resolves the environment, falling back from the interactive login
    /// shell to the account shell and then to the server's own environment.
    /// The server's own variables are removed from every result.
    #[cfg(unix)]
    fn resolve_from_system(&self, server_environment: &Environment) -> ResolvedEnvironment {
        let mut own = without_server_variables(server_environment);
        for (name, value) in &self.shells.account_variables {
            own.entry(name.clone()).or_insert_with(|| value.clone());
        }
        let run = |shell: &PathBuf, arguments: &[&str]| {
            login_shell::run_login_shell(shell, arguments, &own, self.shells.timeout)
        };
        let interactive = self
            .shells
            .interactive
            .as_ref()
            .and_then(|shell| run(shell, &INTERACTIVE_LOGIN_ARGUMENTS));
        if let Some(variables) = interactive {
            return resolved(&variables, None);
        }
        let account = self
            .shells
            .account
            .as_ref()
            .and_then(|shell| run(shell, &LOGIN_ARGUMENTS));
        if let Some(variables) = account {
            return resolved(&variables, Some(WARNING_ACCOUNT_SHELL));
        }
        resolved(&own, Some(WARNING_SERVER_ENVIRONMENT))
    }

    /// Builds the environment from the machine and user entries of the
    /// registry, on top of the server's own environment.
    #[cfg(windows)]
    #[allow(
        clippy::unused_self,
        reason = "the registry environment holds no state of its own"
    )]
    fn resolve_from_system(&self, server_environment: &Environment) -> ResolvedEnvironment {
        use crate::services::registry_environment;

        let own = without_server_variables(server_environment);
        match registry_environment::read_registry_environment(&own) {
            Some(variables) => resolved(&variables, None),
            None => resolved(&own, Some(WARNING_REGISTRY)),
        }
    }
}

fn resolved(variables: &Environment, warning: Option<&str>) -> ResolvedEnvironment {
    ResolvedEnvironment {
        variables: without_server_variables(variables),
        warning: warning.map(str::to_owned),
    }
}

/// Removes the server's own variables and the shell session's, and the entries of `PATH` that point
/// into the server's `AppImage`.
fn without_server_variables(environment: &Environment) -> Environment {
    let app_directory = environment.get("APPDIR").filter(|path| !path.is_empty());
    environment
        .iter()
        .filter(|(name, _)| {
            !SERVER_OWN_VARIABLES.contains(&name.as_str())
                && !SHELL_SESSION_VARIABLES.contains(&name.as_str())
                && !name.starts_with(SERVER_OWN_PREFIX)
        })
        .map(|(name, value)| {
            let value = match app_directory {
                Some(directory) if name == "PATH" => path_without(value, directory),
                _ => value.clone(),
            };
            (name.clone(), value)
        })
        .collect()
}

fn path_without(path: &str, directory: &str) -> String {
    std::env::split_paths(path)
        .filter(|entry| !entry.starts_with(directory))
        .filter_map(|entry| entry.into_os_string().into_string().ok())
        .collect::<Vec<_>>()
        .join(if cfg!(windows) { ";" } else { ":" })
}

#[cfg(test)]
#[path = "login_environment.tests.rs"]
mod tests;

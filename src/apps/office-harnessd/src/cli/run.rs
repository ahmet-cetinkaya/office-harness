// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ffi::OsString;
use std::process::ExitCode;

use wire::LOGIN_START_NOT_REGISTERED_EXIT_CODE;

use super::{CliError, Command, LoginStart, SystemLoginItem};
use crate::config::Config;

const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Used by the app at each start; see `LoginStart::register_at_app_start`.
const AT_APP_START_FLAG: &str = "--at-app-start";

/// Runs one subcommand against the real login item and reports on standard
/// output (the result) or standard error (the failure). `rest` holds the
/// arguments after the subcommand.
#[must_use]
pub fn run(command: Command, rest: &[OsString]) -> ExitCode {
    match execute(command, rest) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("office-harnessd: {error}");
            ExitCode::from(error.exit_code())
        }
    }
}

fn execute(command: Command, rest: &[OsString]) -> Result<ExitCode, CliError> {
    let at_app_start = read_options(command, rest)?;
    // The default folder only: the login item starts the server without arguments.
    let config = Config::load(std::iter::empty())?;
    let login_item = SystemLoginItem;
    let login_start = LoginStart::new(&config.data_folder, VERSION, &login_item);
    match command {
        Command::Register => {
            let running = std::env::current_exe().map_err(CliError::NoRunningProgram)?;
            let installed = if at_app_start {
                login_start.register_at_app_start(&running)?
            } else {
                Some(login_start.register(&running)?)
            };
            match installed {
                Some(installed) => println!("registered: {}", installed.display()),
                None => println!("unchanged: this version is registered, or login start is off"),
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Unregister => {
            login_start.unregister()?;
            println!("unregistered");
            Ok(ExitCode::SUCCESS)
        }
        Command::Status if login_start.is_registered()? => {
            println!("registered");
            Ok(ExitCode::SUCCESS)
        }
        Command::Status => {
            println!("not registered");
            Ok(ExitCode::from(LOGIN_START_NOT_REGISTERED_EXIT_CODE))
        }
    }
}

/// Whether `register` got `--at-app-start`; every other argument is refused.
fn read_options(command: Command, rest: &[OsString]) -> Result<bool, CliError> {
    match (command, rest) {
        (_, []) => Ok(false),
        (Command::Register, [flag]) if flag == AT_APP_START_FLAG => Ok(true),
        (_, [unknown, ..]) => Err(CliError::UnknownArgument(
            unknown.to_string_lossy().into_owned(),
        )),
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ffi::OsString;
use std::process::ExitCode;

use wire::LOGIN_START_NOT_REGISTERED_EXIT_CODE;

use super::{CliError, Command, LoginStart, SystemLoginItem};
use crate::config::Config;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Runs one subcommand against the real login item and reports on standard
/// output (the result) or standard error (the failure). `rest` holds the
/// arguments after the subcommand, which are the server's own.
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
    let config = Config::load(rest.iter().cloned())?;
    let login_item = SystemLoginItem;
    let login_start = LoginStart::new(&config.data_folder, VERSION, &login_item);
    match command {
        Command::Register => {
            let running = std::env::current_exe().map_err(CliError::NoRunningProgram)?;
            let installed = login_start.register(&running)?;
            println!("registered: {}", installed.display());
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

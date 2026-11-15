// SPDX-License-Identifier: AGPL-3.0-or-later

use std::env;
use std::process::ExitCode;

use office_harnessd::bootstrap::{self, ServerLog, StartupError};
use office_harnessd::config::Config;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("office-harnessd: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), StartupError> {
    let config = Config::load(env::args_os().skip(1))?;
    let log = bootstrap::open_log(&config)?;
    let outcome = start_and_wait(&config, &log).await;
    if let Err(error) = &outcome {
        log.error(&error.to_string());
    }
    outcome
}

async fn start_and_wait(config: &Config, log: &ServerLog) -> Result<(), StartupError> {
    let _instance = bootstrap::acquire_single_instance(config)?;
    let _services = bootstrap::start(config, log)?;
    log.info("server started");
    bootstrap::wait_for_shutdown_signal().await?;
    log.info("server stopped");
    Ok(())
}

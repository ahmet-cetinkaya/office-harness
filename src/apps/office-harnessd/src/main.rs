// SPDX-License-Identifier: AGPL-3.0-or-later

// The server has no console window on Windows; messages go to `server.log`
// and the exit code (D13). Unverified on Windows: the process tests read the
// second server's stderr, which the parent passes down as an explicit handle.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::env;
use std::process::ExitCode;

use office_harnessd::bootstrap::{self, ServerLog, StartupError};
use office_harnessd::config::Config;
use office_harnessd::server::{Server, ServerOptions};
use tokio::sync::oneshot;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("office-harnessd: {error}");
            ExitCode::from(error.exit_code())
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
    let services = bootstrap::start(config, log)?;
    let server = Server::bind(config, &services, log, ServerOptions::default()).await?;
    log.info(&format!("server started on port {}", server.port()));
    serve_until_signal(server).await?;
    log.info("server stopped");
    Ok(())
}

/// Serves until the OS asks the server to stop. A signal handler that cannot
/// be installed stops the server and is reported as its own failure.
async fn serve_until_signal(server: Server) -> Result<(), StartupError> {
    let (stop, stopped) = oneshot::channel::<()>();
    let serving = server.serve(async move {
        let _ = stopped.await;
    });
    tokio::pin!(serving);
    tokio::select! {
        outcome = &mut serving => Ok(outcome?),
        signal = bootstrap::wait_for_shutdown_signal() => {
            let _ = stop.send(());
            serving.await?;
            signal.map_err(StartupError::SignalHandler)
        }
    }
}

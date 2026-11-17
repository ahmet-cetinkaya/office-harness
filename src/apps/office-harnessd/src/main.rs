// SPDX-License-Identifier: AGPL-3.0-or-later

// The server has no console window on Windows; messages go to `server.log`
// and the exit code (D13). Unverified on Windows: the process tests read the
// second server's stderr, which the parent passes down as an explicit handle.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::env;
use std::process::ExitCode;

use office_harnessd::bootstrap::{self, ServerLog, Services, StartupError};
use office_harnessd::cli;
use office_harnessd::config::Config;
use office_harnessd::server::{RestartSignal, Server, ServerOptions};
use tokio::sync::oneshot;

#[tokio::main]
async fn main() -> ExitCode {
    let arguments: Vec<_> = env::args_os().skip(1).collect();
    if arguments
        .first()
        .is_some_and(|first| first == cli::agent_call::AGENT_CALL)
    {
        return cli::agent_call::run(&arguments[1..]).await;
    }
    if let Some((command, rest)) = cli::split_command(&arguments) {
        return cli::run(command, rest);
    }
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
    serve_until_stopped(server, &services, log).await?;
    log.info("server stopped");
    Ok(())
}

/// Serves until the OS asks the server to stop, or a client sends
/// `server.restart`. The running agents end first, then the connections
/// close, `server.json` goes, and the server exits. A signal handler that
/// cannot be installed stops the server and is reported as its own failure.
async fn serve_until_stopped(
    server: Server,
    services: &Services,
    log: &ServerLog,
) -> Result<(), StartupError> {
    let restart = server.restart_signal();
    let (stop, stopped) = oneshot::channel::<()>();
    let serving = server.serve(async move {
        let _ = stopped.await;
    });
    tokio::pin!(serving);
    tokio::select! {
        outcome = &mut serving => Ok(outcome?),
        reason = wait_for_stop_request(&restart, log) => {
            bootstrap::interrupt_running_agents(services, log).await;
            let _ = stop.send(());
            serving.await?;
            reason
        }
    }
}

async fn wait_for_stop_request(
    restart: &RestartSignal,
    log: &ServerLog,
) -> Result<(), StartupError> {
    tokio::select! {
        signal = bootstrap::wait_for_shutdown_signal() => {
            signal.map_err(StartupError::SignalHandler)
        }
        () = restart.requested() => {
            log.info("restart requested by a client");
            Ok(())
        }
    }
}

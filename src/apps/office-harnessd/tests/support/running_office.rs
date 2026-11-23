// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

use std::path::PathBuf;

use agent_domain::value_objects::SettingKey;
use agent_infrastructure::persistence::{
    LEDGER_TABLE, SCRIPTS, SettingsRepository, SharedConnection,
};
use office_harnessd::bootstrap::{self, LoginSetup, ServerLog, Services};
use office_harnessd::config::Config;
use office_harnessd::server::{Server, ServerOptions};
use tempfile::TempDir;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

/// A server that listens in this test's runtime, on its own data folder.
pub struct RunningOffice {
    pub config: Config,
    pub services: Services,
    pub log: ServerLog,
    pub port: u16,
    pub server: Option<Server>,
    _folder: TempDir,
}

/// A server that serves in a task and stops on `stop`.
pub struct ServingOffice {
    pub office: RunningOffice,
    stop: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

pub async fn bind_office(options: ServerOptions) -> RunningOffice {
    bind_office_with(options, LoginSetup::detect(), None).await
}

/// Binds with the given login environment, and with the quiet period (in
/// milliseconds) stored as a setting before the server starts, when given.
pub async fn bind_office_with(
    options: ServerOptions,
    login: LoginSetup,
    quiet_period_ms: Option<u64>,
) -> RunningOffice {
    let folder = tempfile::tempdir().expect("folder");
    let config = Config {
        data_folder: folder.path().join("office"),
    };
    let log = bootstrap::open_log(&config).expect("log");
    if let Some(quiet_period_ms) = quiet_period_ms {
        store_quiet_period(&config, quiet_period_ms);
    }
    let services = bootstrap::start_with(&config, &log, login).expect("start");
    let server = Server::bind(&config, &services, &log, options)
        .await
        .expect("bind");
    RunningOffice {
        port: server.port(),
        config,
        services,
        log,
        server: Some(server),
        _folder: folder,
    }
}

fn store_quiet_period(config: &Config, quiet_period_ms: u64) {
    let mut connection = bootstrap::open_database(&config.database_path()).expect("database");
    bootstrap::run_migrations(&mut connection, LEDGER_TABLE, SCRIPTS, &[]).expect("migrations");
    let settings = SettingsRepository::new(SharedConnection::new(connection));
    let mut stored = settings.load().expect("settings");
    stored
        .set_from_text(SettingKey::QuietPeriodMs, &quiet_period_ms.to_string())
        .expect("quiet period");
    settings.save(&stored).expect("save");
}

/// Binds and starts serving.
pub async fn serve_office(options: ServerOptions) -> ServingOffice {
    serve_bound(bind_office(options).await)
}

/// Binds with the given login environment and starts serving.
pub async fn serve_office_with(
    options: ServerOptions,
    login: LoginSetup,
    quiet_period_ms: Option<u64>,
) -> ServingOffice {
    serve_bound(bind_office_with(options, login, quiet_period_ms).await)
}

fn serve_bound(mut office: RunningOffice) -> ServingOffice {
    let server = office.server.take().expect("bound server");
    let (stop, stopped) = oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        server
            .serve(async move {
                let _ = stopped.await;
            })
            .await
            .expect("serve");
    });
    ServingOffice { office, stop, task }
}

/// Starts a server again on the data folder of a stopped one, as a restart of
/// the server process does: new services over the same database.
pub async fn serve_again(stopped: RunningOffice, login: LoginSetup) -> ServingOffice {
    serve_again_with(stopped, login, ServerOptions::default()).await
}

/// As `serve_again`, with the options the new server runs with.
pub async fn serve_again_with(
    stopped: RunningOffice,
    login: LoginSetup,
    options: ServerOptions,
) -> ServingOffice {
    let RunningOffice {
        config,
        log,
        _folder: folder,
        ..
    } = stopped;
    let services = bootstrap::start_with(&config, &log, login).expect("restart");
    let server = Server::bind(&config, &services, &log, options)
        .await
        .expect("bind");
    serve_bound(RunningOffice {
        port: server.port(),
        config,
        services,
        log,
        server: Some(server),
        _folder: folder,
    })
}

impl RunningOffice {
    pub fn token(&self) -> String {
        std::fs::read_to_string(self.config.token_path()).expect("token file")
    }

    pub fn log_text(&self) -> String {
        std::fs::read_to_string(self.config.log_path()).expect("log file")
    }

    pub fn server_file_path(&self) -> PathBuf {
        self.config.server_file_path()
    }
}

impl ServingOffice {
    /// Stops the server and waits until it has closed every connection.
    pub async fn stop(self) -> RunningOffice {
        let _ = self.stop.send(());
        self.task.await.expect("server task");
        self.office
    }
}

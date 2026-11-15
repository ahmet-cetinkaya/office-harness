// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

use std::path::PathBuf;

use office_harnessd::bootstrap::{self, ServerLog, Services};
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
    let folder = tempfile::tempdir().expect("folder");
    let config = Config {
        data_folder: folder.path().join("office"),
    };
    let log = bootstrap::open_log(&config).expect("log");
    let services = bootstrap::start(&config, &log).expect("start");
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

/// Binds and starts serving.
pub async fn serve_office(options: ServerOptions) -> ServingOffice {
    let mut office = bind_office(options).await;
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

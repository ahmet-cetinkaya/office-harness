// SPDX-License-Identifier: AGPL-3.0-or-later

//! The core's one link to the office server: it finds the server, proves the
//! token, keeps the socket, and reconnects. The token stays in this module's
//! files and never reaches the webview.

mod backoff;
mod data_folder;
mod server_file;
mod session;
mod socket;
mod state;
mod state_tracker;

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use serde_json::{Map, Value};
use tokio::sync::{Notify, broadcast, mpsc, oneshot, watch};
use tokio::time::{sleep, timeout};

use crate::api_error::ApiError;
use backoff::Backoff;
use server_file::Incompatibility;
use session::{PendingRequest, run_session};
use socket::{Socket, open_authenticated};
use state_tracker::{Failure, state_after_failure};

pub use data_folder::DataFolder;
pub use state::ConnectionState;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const REQUEST_BACKLOG: usize = 64;
const EVENT_BACKLOG: usize = 256;

struct Shared {
    data_folder: DataFolder,
    state: watch::Sender<ConnectionState>,
    events: broadcast::Sender<Value>,
    /// Set while a session runs; requests have nowhere to go otherwise.
    session_requests: Mutex<Option<mpsc::Sender<PendingRequest>>>,
    retry_now: Notify,
}

/// A handle to the connection task. Clones share the one connection.
#[derive(Clone)]
pub struct Connection {
    shared: Arc<Shared>,
}

impl Connection {
    pub fn new(data_folder: DataFolder) -> Self {
        let (state, _initial_receiver) = watch::channel(ConnectionState::Loading);
        let (events, _initial_events) = broadcast::channel(EVENT_BACKLOG);
        Self {
            shared: Arc::new(Shared {
                data_folder,
                state,
                events,
                session_requests: Mutex::new(None),
                retry_now: Notify::new(),
            }),
        }
    }

    pub fn subscribe_state(&self) -> watch::Receiver<ConnectionState> {
        self.shared.state.subscribe()
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<Value> {
        self.shared.events.subscribe()
    }

    /// Skips the wait before the next attempt.
    pub fn retry_now(&self) {
        self.shared.retry_now.notify_one();
    }

    /// Sends one request and waits for its answer.
    pub async fn request(
        &self,
        kind: String,
        fields: Map<String, Value>,
    ) -> Result<Value, ApiError> {
        let sender = self
            .shared
            .current_requests()
            .ok_or_else(ApiError::not_connected)?;
        let (reply, answer) = oneshot::channel();
        sender
            .send(PendingRequest {
                kind,
                fields,
                reply,
            })
            .await
            .map_err(|_| ApiError::not_connected())?;
        // The session drops the reply when the socket ends first.
        answer.await.map_err(|_| ApiError::not_connected())?
    }

    /// True when the state settled within `limit`: the server answered, whether
    /// it accepted the app or not.
    pub async fn wait_until_settled(&self, limit: Duration) -> bool {
        wait_until_settled(self.subscribe_state(), limit).await
    }

    /// Connects, serves, and reconnects for as long as the app runs.
    pub async fn run(self) {
        let shared = self.shared;
        let mut backoff = Backoff::new();
        loop {
            match establish(&shared.data_folder).await {
                Ok(established) => {
                    backoff.reset();
                    serve(&shared, established).await;
                    shared.publish(ConnectionState::Loading);
                }
                Err(failure) => shared.publish(state_after_failure(failure)),
            }
            pause(&shared, &mut backoff).await;
        }
    }
}

impl Shared {
    fn publish(&self, state: ConnectionState) {
        self.state.send_if_modified(|current| {
            let changed = *current != state;
            if changed {
                *current = state;
            }
            changed
        });
    }

    fn current_requests(&self) -> Option<mpsc::Sender<PendingRequest>> {
        self.session_requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn set_requests(&self, sender: Option<mpsc::Sender<PendingRequest>>) {
        *self
            .session_requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = sender;
    }
}

/// An authenticated socket, plus the version problem to show if the server
/// can still be asked to restart.
struct Established {
    socket: Socket,
    incompatibility: Option<Incompatibility>,
}

async fn establish(data_folder: &DataFolder) -> Result<Established, Failure> {
    let server_file = data_folder
        .read_server_file()
        .map_err(|_| Failure::NotReachable)?;
    let incompatibility = server_file.incompatibility(APP_VERSION);
    if let Some(found) = incompatibility.as_ref().filter(|found| !found.can_restart) {
        return Err(Failure::Incompatible(found.clone()));
    }
    let token = data_folder
        .read_token()
        .map_err(|_| Failure::TokenUnavailable)?;
    let socket = open_authenticated(server_file.port, &token).await?;
    Ok(Established {
        socket,
        incompatibility,
    })
}

async fn serve(shared: &Shared, established: Established) {
    let (sender, receiver) = mpsc::channel(REQUEST_BACKLOG);
    shared.set_requests(Some(sender));
    shared.publish(
        established
            .incompatibility
            .map_or(ConnectionState::Connected, ConnectionState::from),
    );
    run_session(established.socket, receiver, &shared.events).await;
    shared.set_requests(None);
}

/// Waits for the next attempt, or less when a retry is asked for, and then
/// starts the backoff over.
async fn pause(shared: &Shared, backoff: &mut Backoff) {
    tokio::select! {
        () = sleep(backoff.next_delay()) => {}
        () = shared.retry_now.notified() => backoff.reset(),
    }
}

async fn wait_until_settled(
    mut receiver: watch::Receiver<ConnectionState>,
    limit: Duration,
) -> bool {
    timeout(limit, receiver.wait_for(ConnectionState::is_settled))
        .await
        .is_ok_and(|outcome| outcome.is_ok())
}

#[cfg(test)]
#[path = "connection.tests.rs"]
mod tests;

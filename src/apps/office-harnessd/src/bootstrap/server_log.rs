// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use private_fs::{create_private_dir_all, open_private_file};

use super::StartupError;
use crate::config::Config;

/// The owner-only `server.log`. Lines hold ids, status, and error kinds, and
/// never the token, terminal output, prompts, or environment values.
#[derive(Clone)]
pub struct ServerLog {
    file: Arc<Mutex<File>>,
    refusals: Arc<Mutex<RefusalWindow>>,
}

/// At most this many refusal lines are written per window, so that a local
/// process that keeps connecting cannot grow the log without limit.
const REFUSAL_LINES_PER_WINDOW: u32 = 20;
const REFUSAL_WINDOW: Duration = Duration::from_secs(60);

/// Counts the refusal lines of the current window.
struct RefusalWindow {
    started: Instant,
    written: u32,
    suppressed: u32,
}

impl RefusalWindow {
    fn new(now: Instant) -> Self {
        Self {
            started: now,
            written: 0,
            suppressed: 0,
        }
    }

    /// `Some(n)` when the line may be written, where `n` is the count of lines
    /// that the window which just ended did not write; `None` when the line
    /// must be dropped.
    fn admit(&mut self, now: Instant) -> Option<u32> {
        let mut suppressed_before = 0;
        if now.duration_since(self.started) >= REFUSAL_WINDOW {
            suppressed_before = self.suppressed;
            *self = Self::new(now);
        }
        if self.written >= REFUSAL_LINES_PER_WINDOW {
            self.suppressed += 1;
            return None;
        }
        self.written += 1;
        Some(suppressed_before)
    }
}

/// Creates the private data folder and opens `server.log` for appending.
///
/// # Errors
/// `DataFolder` when the folder or the file cannot be prepared.
pub fn open_log(config: &Config) -> Result<ServerLog, StartupError> {
    create_private_dir_all(&config.data_folder)?;
    let file = open_private_file(&config.log_path(), OpenOptions::new().append(true))?;
    Ok(ServerLog {
        file: Arc::new(Mutex::new(file)),
        refusals: Arc::new(Mutex::new(RefusalWindow::new(Instant::now()))),
    })
}

impl ServerLog {
    pub fn info(&self, message: &str) {
        self.write_line("INFO", message);
    }

    /// A refused connection or upgrade. These lines are rate limited, because
    /// any local process can cause them.
    pub fn refusal(&self, message: &str) {
        let admitted = self
            .refusals
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .admit(Instant::now());
        let Some(suppressed_before) = admitted else {
            return;
        };
        if suppressed_before > 0 {
            self.info(&format!(
                "{suppressed_before} refusal lines were not written in the last window"
            ));
        }
        self.info(message);
    }

    pub fn error(&self, message: &str) {
        self.write_line("ERROR", message);
    }

    fn write_line(&self, level: &str, message: &str) {
        let unix_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_millis());
        let mut file = self.file.lock().unwrap_or_else(PoisonError::into_inner);
        // A log that cannot be written must not stop the server.
        let _ = writeln!(file, "{unix_millis} {level} {message}");
    }
}

#[cfg(test)]
#[path = "server_log.tests.rs"]
mod tests;

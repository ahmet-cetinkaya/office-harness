// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use private_fs::{create_private_dir_all, open_private_file};

use super::StartupError;
use crate::config::Config;

/// The owner-only `server.log`. Lines hold ids, status, and error kinds, and
/// never the token, terminal output, prompts, or environment values.
#[derive(Clone)]
pub struct ServerLog {
    file: Arc<Mutex<File>>,
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
    })
}

impl ServerLog {
    pub fn info(&self, message: &str) {
        self.write_line("INFO", message);
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

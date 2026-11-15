// SPDX-License-Identifier: AGPL-3.0-or-later

use axum::http::HeaderMap;
use axum::http::header::{HOST, ORIGIN};

/// Why an upgrade is refused. The client is told nothing but the status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradeRefusal {
    WrongHost,
    OriginPresent,
}

impl UpgradeRefusal {
    /// The reason for the server log.
    #[must_use]
    pub fn reason(self) -> &'static str {
        match self {
            Self::WrongHost => "the Host header is not 127.0.0.1 with the server port",
            Self::OriginPresent => "the request carries an Origin header",
        }
    }
}

/// The contract's upgrade checks: `Host` must be `127.0.0.1:<port>`, which
/// blocks DNS rebinding, and no `Origin` may be present, because a browser
/// always sends one and the client is not a browser.
///
/// # Errors
/// The first check that failed.
pub fn check_upgrade(headers: &HeaderMap, port: u16) -> Result<(), UpgradeRefusal> {
    let expected = format!("127.0.0.1:{port}");
    let host_is_right = headers
        .get(HOST)
        .is_some_and(|host| host.as_bytes() == expected.as_bytes());
    if !host_is_right {
        return Err(UpgradeRefusal::WrongHost);
    }
    if headers.contains_key(ORIGIN) {
        return Err(UpgradeRefusal::OriginPresent);
    }
    Ok(())
}

#[cfg(test)]
#[path = "upgrade_check.tests.rs"]
mod tests;

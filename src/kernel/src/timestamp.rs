// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};

/// A point in time as milliseconds since the Unix epoch, in UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(i64);

impl Timestamp {
    #[must_use]
    pub const fn from_unix_millis(unix_millis: i64) -> Self {
        Self(unix_millis)
    }

    #[must_use]
    pub const fn as_unix_millis(self) -> i64 {
        self.0
    }
}

#[cfg(test)]
#[path = "timestamp.tests.rs"]
mod tests;

// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;

/// A stored or wire event that no known notification event matches, or an
/// identifier that is not a UUID.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("{0}")]
pub struct NotificationError(pub String);

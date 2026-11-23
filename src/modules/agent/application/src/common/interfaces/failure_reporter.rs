// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::common::errors::PortError;

/// Where a failure goes that happens after its request was answered, so no
/// error reply can carry it: a storage write of a background step.
pub trait FailureReporter: Send + Sync {
    fn report(&self, error: &PortError);

    /// A plain line for the server log, with no alert for the clients: a
    /// step of the background that failed without losing stored data.
    fn note(&self, message: &str);
}

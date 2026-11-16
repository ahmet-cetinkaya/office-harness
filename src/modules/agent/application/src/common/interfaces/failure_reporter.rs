// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::common::errors::PortError;

/// Where a failure goes that happens after its request was answered, so no
/// error reply can carry it: a storage write of a background step.
pub trait FailureReporter: Send + Sync {
    fn report(&self, error: &PortError);
}

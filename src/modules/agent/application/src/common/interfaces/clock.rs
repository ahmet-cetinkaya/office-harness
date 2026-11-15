// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::Timestamp;

pub trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;
}

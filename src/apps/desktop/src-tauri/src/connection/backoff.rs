// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

const FIRST_DELAY: Duration = Duration::from_millis(500);
const LONGEST_DELAY: Duration = Duration::from_secs(8);

/// The wait between connection attempts: it doubles up to a ceiling.
#[derive(Debug)]
pub struct Backoff {
    next: Duration,
}

impl Backoff {
    pub fn new() -> Self {
        Self { next: FIRST_DELAY }
    }

    pub fn next_delay(&mut self) -> Duration {
        let delay = self.next;
        self.next = (delay * 2).min(LONGEST_DELAY);
        delay
    }

    pub fn reset(&mut self) {
        self.next = FIRST_DELAY;
    }
}

#[cfg(test)]
#[path = "backoff.tests.rs"]
mod tests;

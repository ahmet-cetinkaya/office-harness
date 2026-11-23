// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

/// A fake with exactly the methods of the port. It compiles only while the
/// traits have no required method that ends a process by its id (plan M1).
struct GoneHandle;

impl ProcessHandle for GoneHandle {
    fn identity(&self) -> Result<ProcessIdentity, ReadError> {
        Err(ReadError::NotRunning)
    }

    fn request_end(&self) -> Result<EndOutcome, String> {
        Ok(EndOutcome::Gone)
    }

    fn force_end(&self) -> Result<EndOutcome, String> {
        Ok(EndOutcome::Gone)
    }

    fn wait_for_exit(&self, _timeout: Duration) -> bool {
        true
    }
}

struct EmptyInspector;

impl ProcessInspector for EmptyInspector {
    fn open(&self, _pid: u32) -> Option<Box<dyn ProcessHandle>> {
        Some(Box::new(GoneHandle))
    }

    fn boot(&self) -> Option<String> {
        None
    }

    fn descendants(&self, _root: u32) -> Vec<u32> {
        Vec::new()
    }
}

#[test]
fn the_port_ends_only_through_a_handle() {
    let handle = EmptyInspector.open(1).expect("the fake opens every pid");
    assert_eq!(handle.force_end(), Ok(EndOutcome::Gone));
}

// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn the_lagged_frame_is_a_dropped_frame_for_the_nil_agent() {
    let frame = lagged_frame();
    assert_eq!(frame.len(), 17);
    assert_eq!(frame[0], 0x03);
    assert!(frame[1..].iter().all(|byte| *byte == 0));
}

#[test]
fn the_resync_event_has_the_client_type() {
    assert_eq!(resync_event()["type"], "client.resync");
}

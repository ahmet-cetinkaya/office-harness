// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn cursor_reply_is_one_based() {
    assert_eq!(TerminalQuery::CursorPosition.reply(0, 0), b"\x1b[1;1R");
    assert_eq!(TerminalQuery::CursorPosition.reply(4, 9), b"\x1b[5;10R");
}

#[test]
fn every_query_has_a_reply() {
    for query in [
        TerminalQuery::DeviceAttributes,
        TerminalQuery::SecondaryDeviceAttributes,
        TerminalQuery::DeviceStatus,
    ] {
        assert_ne!(query.reply(0, 0), Vec::<u8>::new());
    }
}

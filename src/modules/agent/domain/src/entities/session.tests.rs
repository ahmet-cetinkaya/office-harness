// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn a_new_session_is_running_until_it_has_an_end() {
    let mut session = Session::begin(
        SessionId::generate(),
        AgentId::generate(),
        None,
        None,
        OfficeAdditions::default(),
        Timestamp::from_unix_millis(1),
    );
    assert!(session.is_running());
    session.end(ExitResult::Code(0), Timestamp::from_unix_millis(2));
    assert!(!session.is_running());
}

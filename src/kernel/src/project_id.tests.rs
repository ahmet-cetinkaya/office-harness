// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn round_trips_through_text() {
    let project_id = ProjectId::generate();
    assert_eq!(project_id.to_string().parse::<ProjectId>(), Ok(project_id));
}

#[test]
fn rejects_text_that_is_not_a_uuid() {
    assert_eq!(
        "not-an-id".parse::<ProjectId>(),
        Err(ProjectIdError("not-an-id".to_owned()))
    );
}

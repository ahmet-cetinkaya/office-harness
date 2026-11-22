// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn the_title_carries_the_count_only_when_an_agent_needs_the_user() {
    assert_eq!(window_title(0), "office-harness");
    assert_eq!(window_title(3), "office-harness (3)");
}

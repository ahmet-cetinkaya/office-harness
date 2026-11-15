// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn refusal_lines_stop_at_the_limit_and_report_the_drop_in_the_next_window() {
    let start = Instant::now();
    let mut window = RefusalWindow::new(start);
    for _ in 0..REFUSAL_LINES_PER_WINDOW {
        assert_eq!(window.admit(start), Some(0));
    }
    assert_eq!(window.admit(start), None);
    assert_eq!(window.admit(start), None);

    assert_eq!(window.admit(start + REFUSAL_WINDOW), Some(2));
    assert_eq!(window.admit(start + REFUSAL_WINDOW), Some(0));
}

// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn delays_double_up_to_the_ceiling() {
    let mut backoff = Backoff::new();

    let delays: Vec<Duration> = (0..7).map(|_| backoff.next_delay()).collect();

    let seconds: Vec<f64> = delays.iter().map(Duration::as_secs_f64).collect();
    assert_eq!(seconds, [0.5, 1.0, 2.0, 4.0, 8.0, 8.0, 8.0]);
}

#[test]
fn reset_starts_over() {
    let mut backoff = Backoff::new();
    backoff.next_delay();
    backoff.next_delay();

    backoff.reset();

    assert_eq!(backoff.next_delay(), FIRST_DELAY);
}

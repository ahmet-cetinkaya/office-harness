// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn every_kind_reads_back_from_its_stored_name() {
    for kind in AttentionKind::ALL {
        assert_eq!(kind.as_str().parse::<AttentionKind>(), Ok(kind));
    }
}

#[test]
fn an_unknown_name_is_rejected() {
    assert!("sleeping".parse::<AttentionKind>().is_err());
}

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

#[test]
fn a_resume_hold_is_stored_as_resume_hold() {
    assert_eq!(AttentionKind::ResumeHold.as_str(), "resume_hold");
}

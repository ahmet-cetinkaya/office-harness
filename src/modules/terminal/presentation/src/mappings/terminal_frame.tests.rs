// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn agent() -> (AgentId, [u8; 16]) {
    let id: AgentId = "7d0c4a52-1f3e-4b7a-9c11-0a1b2c3d4e5f"
        .parse()
        .expect("uuid");
    let bytes = *Uuid::parse_str(&id.to_string()).expect("uuid").as_bytes();
    (id, bytes)
}

#[test]
fn a_frame_is_kind_then_uuid_then_payload() {
    let (id, uuid) = agent();
    let frame = TerminalFrame::Snapshot(b"abc".to_vec())
        .encode(id)
        .expect("encode");
    assert_eq!(frame[0], 0x01);
    assert_eq!(frame[1..17], uuid);
    assert_eq!(&frame[17..], b"abc");
}

#[test]
fn each_kind_has_its_byte() {
    let (id, _) = agent();
    let kind = |frame: TerminalFrame| frame.encode(id).expect("encode")[0];
    assert_eq!(kind(TerminalFrame::Output(Vec::new())), 0x02);
    assert_eq!(kind(TerminalFrame::OlderOutputDropped), 0x03);
}

#[test]
fn the_dropped_notice_has_no_payload() {
    let (id, _) = agent();
    let frame = TerminalFrame::from_live(LiveOutput::OlderOutputDropped)
        .encode(id)
        .expect("encode");
    assert_eq!(frame.len(), 17);
}

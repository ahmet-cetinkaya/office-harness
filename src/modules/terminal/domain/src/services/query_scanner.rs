// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::value_objects::TerminalQuery;

const ESCAPE: u8 = 0x1b;
/// No query has more parameter bytes than this; a longer sequence is not one.
const MAX_PARAMETER_BYTES: usize = 8;

#[derive(Debug, Default)]
enum ScanState {
    #[default]
    Ground,
    Escape,
    ControlSequence(Vec<u8>),
}

/// A query found in a chunk. `end` is the offset just after its last byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoundQuery {
    pub end: usize,
    pub query: TerminalQuery,
}

/// Finds terminal queries in output. It keeps its state between chunks, so a
/// query split across two reads is found once, in the chunk that ends it.
#[derive(Debug, Default)]
pub struct QueryScanner {
    state: ScanState,
}

impl QueryScanner {
    pub fn scan(&mut self, bytes: &[u8]) -> Vec<FoundQuery> {
        bytes
            .iter()
            .enumerate()
            .filter_map(|(index, &byte)| {
                self.advance(byte).map(|query| FoundQuery {
                    end: index + 1,
                    query,
                })
            })
            .collect()
    }

    fn advance(&mut self, byte: u8) -> Option<TerminalQuery> {
        if byte == ESCAPE {
            self.state = ScanState::Escape;
            return None;
        }
        match std::mem::take(&mut self.state) {
            ScanState::Ground => None,
            ScanState::Escape => {
                if byte == b'[' {
                    self.state = ScanState::ControlSequence(Vec::new());
                }
                None
            }
            ScanState::ControlSequence(mut parameters) => match byte {
                0x20..=0x3f => {
                    if parameters.len() < MAX_PARAMETER_BYTES {
                        parameters.push(byte);
                        self.state = ScanState::ControlSequence(parameters);
                    }
                    None
                }
                0x40..=0x7e => classify(&parameters, byte),
                _ => None,
            },
        }
    }
}

fn classify(parameters: &[u8], final_byte: u8) -> Option<TerminalQuery> {
    match (parameters, final_byte) {
        (b"" | b"0", b'c') => Some(TerminalQuery::DeviceAttributes),
        (b">" | b">0", b'c') => Some(TerminalQuery::SecondaryDeviceAttributes),
        (b"6", b'n') => Some(TerminalQuery::CursorPosition),
        (b"5", b'n') => Some(TerminalQuery::DeviceStatus),
        _ => None,
    }
}

#[cfg(test)]
#[path = "query_scanner.tests.rs"]
mod tests;

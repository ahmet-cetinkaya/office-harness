// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reads the output of the git commands of the file view (0016 D6): status,
//! name-status, log, and diff text. Pure, so each form is tested without git.

use agent_application::common::models::ChangeKind;

const FIELD_SEPARATOR: char = '\u{1f}';
const RECORD_SEPARATOR: u8 = 0x1e;
const HUNK_START: &[u8] = b"@@ ";
const BINARY_LINE: &[u8] = b"Binary files ";
const NO_NEWLINE_NOTE: &str = "\\ No newline at end of file\n";
const SUBMODULE_MARK: u8 = b'S';
/// `1 XY sub mH mI mW hH hI path`: eight fields, then the path.
const ORDINARY_FIELDS: usize = 9;
/// `2 XY sub mH mI mW hH hI Xscore path`, then the old path in the next
/// record.
const RENAMED_FIELDS: usize = 10;
/// `u XY sub m1 m2 m3 mW h1 h2 h3 path`.
const UNMERGED_FIELDS: usize = 11;
const LOG_FIELDS: usize = 5;

/// One changed path of git's status or name-status output, as git names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeRecord {
    pub kind: ChangeKind,
    pub path: String,
    pub old_path: Option<String>,
}

/// One commit of the history output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRecord {
    pub id: String,
    pub short_id: String,
    pub author: String,
    pub author_seconds: i64,
    pub subject: String,
}

/// What a diff output holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffBody {
    /// No line of change.
    Empty,
    Binary,
    /// The change text starts at this byte.
    Text(usize),
}

/// The records of `status --porcelain=v2 -z`: changed, renamed, unmerged,
/// and untracked paths. Ignored paths and headers are skipped.
#[must_use]
pub fn parse_status(output: &[u8]) -> Vec<ChangeRecord> {
    let mut records = output
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty());
    let mut changes = Vec::new();
    while let Some(record) = records.next() {
        let change = match record.first() {
            Some(b'1') => ordinary_record(record),
            Some(b'2') => renamed_record(record, records.next()),
            Some(b'u') => unmerged_record(record),
            Some(b'?') => Some(untracked_record(record)),
            _ => None,
        };
        changes.extend(change);
    }
    changes
}

/// The records of `diff-tree --name-status -z`.
#[must_use]
pub fn parse_name_status(output: &[u8]) -> Vec<ChangeRecord> {
    let mut parts = output
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty());
    let mut changes = Vec::new();
    while let Some(status) = parts.next() {
        let Some(path) = parts.next() else { break };
        let kind = match status.first() {
            Some(b'A') => ChangeKind::Added,
            Some(b'D') => ChangeKind::Deleted,
            Some(b'T') => ChangeKind::TypeChanged,
            Some(b'R' | b'C') => ChangeKind::Renamed,
            _ => ChangeKind::Modified,
        };
        if kind == ChangeKind::Renamed {
            let Some(new_path) = parts.next() else { break };
            changes.push(ChangeRecord {
                kind,
                path: text(new_path),
                old_path: Some(text(path)),
            });
        } else {
            changes.push(ChangeRecord {
                kind,
                path: text(path),
                old_path: None,
            });
        }
    }
    changes
}

/// The commits of `log --format=%H%x1f%h%x1f%an%x1f%at%x1f%s%x1e`.
#[must_use]
pub fn parse_log(output: &[u8]) -> Vec<LogRecord> {
    output
        .split(|byte| *byte == RECORD_SEPARATOR)
        .filter_map(|record| {
            let record = text(record.trim_ascii_start());
            let mut fields = record.splitn(LOG_FIELDS, FIELD_SEPARATOR);
            Some(LogRecord {
                id: fields.next().filter(|id| !id.is_empty())?.to_owned(),
                short_id: fields.next()?.to_owned(),
                author: fields.next()?.to_owned(),
                author_seconds: fields.next()?.parse().ok()?,
                subject: fields.next()?.to_owned(),
            })
        })
        .collect()
}

/// Where the change text of a diff starts, or that it is binary or empty:
/// git's header lines before the first `@@` are not part of it.
#[must_use]
pub fn diff_body(output: &[u8]) -> DiffBody {
    let mut start = 0;
    for line in output.split_inclusive(|byte| *byte == b'\n') {
        if line.starts_with(HUNK_START) {
            return DiffBody::Text(start);
        }
        if line.starts_with(BINARY_LINE) {
            return DiffBody::Binary;
        }
        start += line.len();
    }
    DiffBody::Empty
}

/// The diff of a file git does not track: every line added.
#[must_use]
pub fn added_file_diff(content: &[u8]) -> Vec<u8> {
    if content.is_empty() {
        return Vec::new();
    }
    let has_final_newline = content.ends_with(b"\n");
    let body = content.strip_suffix(b"\n").unwrap_or(content);
    let lines: Vec<&[u8]> = body.split(|byte| *byte == b'\n').collect();
    let count = match lines.len() {
        1 => "1".to_owned(),
        many => format!("1,{many}"),
    };
    let mut diff = format!("@@ -0,0 +{count} @@\n").into_bytes();
    for line in lines {
        diff.push(b'+');
        diff.extend_from_slice(line);
        diff.push(b'\n');
    }
    if !has_final_newline {
        diff.extend_from_slice(NO_NEWLINE_NOTE.as_bytes());
    }
    diff
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn fields(record: &[u8], count: usize) -> Vec<&[u8]> {
    record.splitn(count, |byte| *byte == b' ').collect()
}

fn ordinary_record(record: &[u8]) -> Option<ChangeRecord> {
    let [_, status, submodule, .., path] = fields(record, ORDINARY_FIELDS)[..] else {
        return None;
    };
    let kind = if submodule.first() == Some(&SUBMODULE_MARK) {
        ChangeKind::Submodule
    } else {
        kind_of_status(status)
    };
    Some(ChangeRecord {
        kind,
        path: text(path),
        old_path: None,
    })
}

fn renamed_record(record: &[u8], old_path: Option<&[u8]>) -> Option<ChangeRecord> {
    let path = *fields(record, RENAMED_FIELDS).last()?;
    Some(ChangeRecord {
        kind: ChangeKind::Renamed,
        path: text(path),
        old_path: old_path.map(text),
    })
}

fn unmerged_record(record: &[u8]) -> Option<ChangeRecord> {
    let path = *fields(record, UNMERGED_FIELDS).last()?;
    Some(ChangeRecord {
        kind: ChangeKind::Conflicted,
        path: text(path),
        old_path: None,
    })
}

fn untracked_record(record: &[u8]) -> ChangeRecord {
    ChangeRecord {
        kind: ChangeKind::Untracked,
        path: text(record.get(2..).unwrap_or_default()),
        old_path: None,
    }
}

/// The kind of the two status letters (index, work tree) against HEAD.
fn kind_of_status(status: &[u8]) -> ChangeKind {
    match status {
        [b'D', _] | [_, b'D'] => ChangeKind::Deleted,
        [b'A', _] | [_, b'A'] => ChangeKind::Added,
        [b'T', _] | [_, b'T'] => ChangeKind::TypeChanged,
        _ => ChangeKind::Modified,
    }
}

#[cfg(test)]
#[path = "git_parsing.tests.rs"]
mod tests;

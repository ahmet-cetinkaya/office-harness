// SPDX-License-Identifier: AGPL-3.0-or-later

const TEAM_TOKEN: &str = "{team}";
const NUMBER_TOKEN: &str = "{n}";
const FIRST_NUMBER: u64 = 1;

/// Default pattern of a team's lead entry (A2).
pub const LEAD_PATTERN: &str = "{team}-lead";
/// Default pattern of a team's member entry (A2).
pub const MEMBER_PATTERN: &str = "{team}-member-{n}";

/// The default pattern of a template copied from the agent `name` (A2).
#[must_use]
pub fn default_agent_pattern(name: &str) -> String {
    format!("{name}-{NUMBER_TOKEN}")
}

/// The text from which the office makes an agent's name (D4). `{team}` is the
/// team name and `{n}` the lowest number that makes the name unique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamePattern(String);

enum Segment<'a> {
    Literal(&'a str),
    Team,
    Number,
}

impl NamePattern {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.trim().is_empty()
    }

    #[must_use]
    pub fn needs_team(&self) -> bool {
        segments(&self.0)
            .iter()
            .any(|segment| matches!(segment, Segment::Team))
    }

    /// The length of the name when the team name is empty and the number is
    /// the first one: the least length that any creation can give, which is
    /// what a save can check before a team name is known.
    #[must_use]
    pub fn least_expanded_chars(&self) -> usize {
        self.render("", FIRST_NUMBER).chars().count()
    }

    /// The first name that `is_taken` does not refuse (D4): the lowest number
    /// for `{n}`; for a pattern without `{n}`, the text itself, and with
    /// `-{n}` added only when that is taken. `None` when the pattern has
    /// `{team}` and `team` is `None`.
    pub fn unique_name(
        &self,
        team: Option<&str>,
        is_taken: impl Fn(&str) -> bool,
    ) -> Option<String> {
        if self.needs_team() && team.is_none() {
            return None;
        }
        let team = team.unwrap_or_default();
        let numbered = if self.has_number() {
            self.clone()
        } else {
            let plain = self.render(team, FIRST_NUMBER);
            if !is_taken(&plain) {
                return Some(plain);
            }
            Self(format!("{}-{NUMBER_TOKEN}", self.0))
        };
        let mut number = FIRST_NUMBER;
        loop {
            let name = numbered.render(team, number);
            if !is_taken(&name) {
                return Some(name);
            }
            number += 1;
        }
    }

    fn has_number(&self) -> bool {
        segments(&self.0)
            .iter()
            .any(|segment| matches!(segment, Segment::Number))
    }

    /// One pass over the pattern, so a team name that holds `{n}` stays text.
    fn render(&self, team: &str, number: u64) -> String {
        segments(&self.0)
            .iter()
            .map(|segment| match segment {
                Segment::Literal(text) => (*text).to_owned(),
                Segment::Team => team.to_owned(),
                Segment::Number => number.to_string(),
            })
            .collect()
    }
}

fn segments(text: &str) -> Vec<Segment<'_>> {
    let mut segments = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let (segment, length) = next_segment(rest);
        segments.push(segment);
        rest = &rest[length..];
    }
    segments
}

fn next_segment(rest: &str) -> (Segment<'_>, usize) {
    if rest.starts_with(TEAM_TOKEN) {
        return (Segment::Team, TEAM_TOKEN.len());
    }
    if rest.starts_with(NUMBER_TOKEN) {
        return (Segment::Number, NUMBER_TOKEN.len());
    }
    let first_length = rest.chars().next().map_or(0, char::len_utf8);
    let end = rest[first_length..]
        .find('{')
        .map_or(rest.len(), |at| at + first_length);
    (Segment::Literal(&rest[..end]), end)
}

#[cfg(test)]
#[path = "name_pattern.tests.rs"]
mod tests;

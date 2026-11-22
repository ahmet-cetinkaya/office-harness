// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::entities::AgentBody;
use crate::errors::TemplateError;

/// The most agents one team template creates, the lead included (FR-010).
pub const MAX_TEAM_AGENTS: u64 = 50;
const MINIMUM_COUNT: u32 = 1;

/// A member entry: a copy of an agent definition and how many members it
/// makes (A5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberEntry {
    pub entry: AgentBody,
    pub count: u32,
}

/// A saved team definition: one lead entry and one or more member entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamBody {
    pub lead: AgentBody,
    pub members: Vec<MemberEntry>,
}

impl TeamBody {
    /// The FR-003 and FR-010 save rules.
    ///
    /// # Errors
    /// The refusal of the first entry that breaks a rule, `CountBelowOne`,
    /// `NoMembers`, or `TooManyAgents`.
    pub fn validate(&self) -> Result<(), TemplateError> {
        self.lead.validate("lead")?;
        if self.members.is_empty() {
            return Err(TemplateError::NoMembers);
        }
        for (index, member) in self.members.iter().enumerate() {
            member.entry.validate(&format!("members[{index}].entry"))?;
            if member.count < MINIMUM_COUNT {
                return Err(TemplateError::CountBelowOne { index });
            }
        }
        let total = self.agent_count();
        if total > MAX_TEAM_AGENTS {
            return Err(TemplateError::TooManyAgents {
                total,
                max: MAX_TEAM_AGENTS,
            });
        }
        Ok(())
    }

    /// The lead plus every member the counts make.
    #[must_use]
    pub fn agent_count(&self) -> u64 {
        1 + self
            .members
            .iter()
            .map(|member| u64::from(member.count))
            .sum::<u64>()
    }

    #[must_use]
    pub fn without_env_values(self) -> Self {
        Self {
            lead: self.lead.without_env_values(),
            members: self
                .members
                .into_iter()
                .map(|member| MemberEntry {
                    entry: member.entry.without_env_values(),
                    count: member.count,
                })
                .collect(),
        }
    }
}

#[cfg(test)]
#[path = "team_body.tests.rs"]
mod tests;

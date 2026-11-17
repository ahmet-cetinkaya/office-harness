// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AgentError;

macro_rules! uuid_identifier {
    ($name:ident, $field:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            #[must_use]
            pub fn generate() -> Self {
                Self(Uuid::new_v4())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }

        impl FromStr for $name {
            type Err = AgentError;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(text)
                    .map(Self)
                    .map_err(|_| AgentError::InvalidId {
                        field: $field,
                        text: text.to_owned(),
                    })
            }
        }

        impl $name {
            /// Reads the id as stored text. `None` for any text that is not
            /// a UUID.
            #[must_use]
            pub fn from_uuid_text(text: &str) -> Option<Self> {
                Uuid::parse_str(text).ok().map(Self)
            }

            #[must_use]
            pub fn as_uuid_text(&self) -> String {
                self.0.to_string()
            }
        }
    };
}

uuid_identifier!(MachineId, "machine id");
uuid_identifier!(SessionId, "session id");

#[cfg(test)]
#[path = "entity_ids.tests.rs"]
mod tests;

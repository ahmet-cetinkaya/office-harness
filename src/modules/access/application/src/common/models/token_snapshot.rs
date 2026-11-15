// SPDX-License-Identifier: AGPL-3.0-or-later

use access_domain::value_objects::OfficeToken;

/// The token and its generation, which starts at 1 and grows by one at each
/// regeneration. A connection remembers the generation it authenticated with.
#[derive(Debug, Clone)]
pub struct TokenSnapshot {
    pub token: OfficeToken,
    pub generation: u64,
}

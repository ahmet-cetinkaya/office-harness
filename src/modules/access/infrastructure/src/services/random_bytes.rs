// SPDX-License-Identifier: AGPL-3.0-or-later

use access_application::common::errors::AccessError;
use access_domain::value_objects::RANDOM_BYTE_COUNT;

/// Bytes from the operating system's secure random source.
pub(super) fn random_bytes() -> Result<[u8; RANDOM_BYTE_COUNT], AccessError> {
    let mut bytes = [0_u8; RANDOM_BYTE_COUNT];
    getrandom::fill(&mut bytes).map_err(|error| AccessError::Randomness(error.to_string()))?;
    Ok(bytes)
}

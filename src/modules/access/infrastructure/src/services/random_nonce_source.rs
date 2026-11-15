// SPDX-License-Identifier: AGPL-3.0-or-later

use access_application::common::errors::AccessError;
use access_application::common::interfaces::NonceSource;
use access_domain::value_objects::Nonce;

use super::random_bytes::random_bytes;

#[derive(Debug, Clone, Copy, Default)]
pub struct RandomNonceSource;

impl NonceSource for RandomNonceSource {
    fn next_nonce(&self) -> Result<Nonce, AccessError> {
        Ok(Nonce::from_random_bytes(random_bytes()?))
    }
}

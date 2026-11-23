// SPDX-License-Identifier: AGPL-3.0-or-later

use secret_domain::StoreKey;

/// A new random store key from the OS random source.
///
/// # Errors
/// The OS random source's error text.
pub fn generate_store_key() -> Result<StoreKey, String> {
    let mut bytes = [0_u8; StoreKey::LENGTH];
    getrandom::fill(&mut bytes).map_err(|error| error.to_string())?;
    let key = StoreKey::from_bytes(bytes);
    zeroize::Zeroize::zeroize(&mut bytes);
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_generated_keys_differ() {
        let first = generate_store_key().expect("key");
        let second = generate_store_key().expect("key");
        assert!(!first.matches(&second));
    }
}

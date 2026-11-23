// SPDX-License-Identifier: AGPL-3.0-or-later

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use secret_application::{RecordSealer, SealError};
use secret_domain::{RecordBinding, SealedRecord, SecretValue, StoreKey};

const NONCE_BYTES: usize = 24;

/// Seals with XChaCha20-Poly1305: a fresh random 192-bit nonce for every seal,
/// the binding as authenticated data. The record is the nonce then the
/// ciphertext with its tag.
#[derive(Debug, Default)]
pub struct ChaChaRecordSealer;

impl RecordSealer for ChaChaRecordSealer {
    fn seal(
        &self,
        key: &StoreKey,
        binding: &RecordBinding,
        value: &SecretValue,
    ) -> Result<SealedRecord, SealError> {
        let mut nonce = [0_u8; NONCE_BYTES];
        getrandom::fill(&mut nonce).map_err(|_| SealError::CannotSeal)?;
        let ciphertext = XChaCha20Poly1305::new(key.expose().into())
            .encrypt(
                &XNonce::from(nonce),
                Payload {
                    msg: value.reveal().as_bytes(),
                    aad: &binding.authenticated_data(),
                },
            )
            .map_err(|_| SealError::CannotSeal)?;
        let mut record = Vec::with_capacity(NONCE_BYTES + ciphertext.len());
        record.extend_from_slice(&nonce);
        record.extend_from_slice(&ciphertext);
        Ok(SealedRecord::from_bytes(record))
    }

    fn open(
        &self,
        key: &StoreKey,
        binding: &RecordBinding,
        record: &SealedRecord,
    ) -> Result<SecretValue, SealError> {
        let bytes = record.as_bytes();
        if bytes.len() < NONCE_BYTES {
            return Err(SealError::CannotOpen);
        }
        let (nonce, ciphertext) = bytes.split_at(NONCE_BYTES);
        let nonce = XNonce::try_from(nonce).map_err(|_| SealError::CannotOpen)?;
        let plain = XChaCha20Poly1305::new(key.expose().into())
            .decrypt(
                &nonce,
                Payload {
                    msg: ciphertext,
                    aad: &binding.authenticated_data(),
                },
            )
            .map_err(|_| SealError::CannotOpen)?;
        let plain = zeroize::Zeroizing::new(plain);
        let text = String::from_utf8(plain.to_vec()).map_err(|_| SealError::CannotOpen)?;
        SecretValue::parse(text).map_err(|_| SealError::CannotOpen)
    }
}

#[cfg(test)]
#[path = "chacha_sealer.tests.rs"]
mod tests;

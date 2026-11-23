// SPDX-License-Identifier: AGPL-3.0-or-later

//! Secret module domain (0026): names, values, versions, the store key, the
//! custody of that key, and the binding of a sealed record. No I/O. Nothing
//! here prints a secret value or the store key: their `Debug` shows `[secret]`.

mod entities;
mod errors;
mod value_objects;

pub use entities::{Custody, LockedReason};
pub use errors::SecretError;
pub use value_objects::{
    MAX_SECRETS, MAX_VALUE_BYTES, MIN_VALUE_BYTES, RecordBinding, SealedRecord, SecretId,
    SecretName, SecretValue, SecretVersion, StoreKey,
};

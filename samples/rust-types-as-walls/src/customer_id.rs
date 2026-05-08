//! newtype の ergonomics を `From` / `TryFrom` / `serde(transparent)` で整える。

use std::num::NonZeroU64;

use derive_more::{AsRef, Display, From};
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, From, Display, AsRef)]
#[serde(transparent)]
#[display("{_0}")]
pub struct CustomerId(#[as_ref] NonZeroU64);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CustomerIdError {
    #[error("customer_id は 1 以上でなければなりません")]
    Zero,
}

impl CustomerId {
    pub fn get(self) -> u64 {
        self.0.get()
    }
}

impl TryFrom<u64> for CustomerId {
    type Error = CustomerIdError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        NonZeroU64::new(value)
            .map(Self::from)
            .ok_or(CustomerIdError::Zero)
    }
}

impl From<CustomerId> for u64 {
    fn from(value: CustomerId) -> Self {
        value.get()
    }
}

impl<'de> Deserialize<'de> for CustomerId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = u64::deserialize(deserializer)?;
        Self::try_from(raw).map_err(serde::de::Error::custom)
    }
}

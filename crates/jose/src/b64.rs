// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use std::ops::{Deref, DerefMut};

use drawbridge_byte::UrlSafeNoPad;

use serde::de::DeserializeOwned;
use serde::{ser::Error as _, Deserialize, Serialize};

pub type Bytes<T = Vec<u8>, C = UrlSafeNoPad> = drawbridge_byte::Bytes<T, C>;

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Json<T>(pub T);

impl<T> From<T> for Json<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Json<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Serialize> Serialize for Json<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let buf = serde_json::to_vec(self).map_err(|_| S::Error::custom("encoding error"))?;
        Bytes::<_, UrlSafeNoPad>::from(buf).serialize(serializer)
    }
}

impl<'de, T: DeserializeOwned> Deserialize<'de> for Json<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let buf = Bytes::<Vec<u8>>::deserialize(deserializer)?;
        let val = serde_json::from_slice(&buf).unwrap();
        Ok(Self(val))
    }
}

// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use serde::{de::Error as _, Deserialize, Serialize};

pub trait Config {
    const CONFIG: base64::Config;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Standard(());

impl Config for Standard {
    const CONFIG: base64::Config = base64::STANDARD;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UrlSafeNoPad(());

impl Config for UrlSafeNoPad {
    const CONFIG: base64::Config = base64::URL_SAFE_NO_PAD;
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bytes<T = Vec<u8>, C = Standard>(T, PhantomData<C>);

impl<T, C> Bytes<T, C> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T, C> From<T> for Bytes<T, C> {
    fn from(value: T) -> Self {
        Self(value, PhantomData)
    }
}

impl<T, C> Deref for Bytes<T, C> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, C> DerefMut for Bytes<T, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: AsRef<[u8]>, C: Config> std::fmt::Display for Bytes<T, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&base64::encode_config(self.0.as_ref(), C::CONFIG))
    }
}

impl<T: From<Vec<u8>>, C: Config> FromStr for Bytes<T, C> {
    type Err = base64::DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        base64::decode_config(s, C::CONFIG).map(|x| Self(x.into(), PhantomData))
    }
}

impl<T: AsRef<[u8]>, C: Config> Serialize for Bytes<T, C> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            base64::encode_config(self.0.as_ref(), C::CONFIG).serialize(serializer)
        } else {
            serializer.serialize_bytes(self.0.as_ref())
        }
    }
}

impl<'de, T: From<Vec<u8>>, C: Config> Deserialize<'de> for Bytes<T, C> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            let b64 = String::deserialize(deserializer)?;
            let buf = base64::decode_config(&b64, C::CONFIG)
                .map_err(|_| D::Error::custom("invalid base64"))?;
            Ok(Self(buf.into(), PhantomData))
        } else {
            Ok(Self(Vec::deserialize(deserializer)?.into(), PhantomData))
        }
    }
}

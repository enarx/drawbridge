// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

//! This crate provides a [`Bytes`] type which wraps most types that represent
//! a contiguous array of bytes. It provides implementations for easy
//! conversions to and from Base64 representations in string contexts.

use std::borrow::Cow;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use serde::{de::Error as _, Deserialize, Serialize};

mod sealed {
    pub trait Config {
        const CONFIG: base64::Config;
    }
}

use sealed::Config;

/// Standard Base64 encoding with padding
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Standard(());

impl Config for Standard {
    const CONFIG: base64::Config = base64::STANDARD;
}

/// Standard Base64 encoding without padding
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StandardNoPad(());

impl Config for StandardNoPad {
    const CONFIG: base64::Config = base64::STANDARD_NO_PAD;
}

/// URL-safe Base64 encoding with padding
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UrlSafe(());

impl Config for UrlSafe {
    const CONFIG: base64::Config = base64::URL_SAFE;
}

/// URL-safe Base64 encoding without padding
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UrlSafeNoPad(());

impl Config for UrlSafeNoPad {
    const CONFIG: base64::Config = base64::URL_SAFE_NO_PAD;
}

/// A wrapper for bytes which provides base64 encoding in string contexts
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bytes<T, C = Standard>(T, PhantomData<C>);

impl<T: Debug, C> Debug for Bytes<T, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Bytes").field(&self.0).finish()
    }
}

impl<T: Default, C> Default for Bytes<T, C> {
    fn default() -> Self {
        Self(Default::default(), PhantomData)
    }
}

impl<T, C> Bytes<T, C> {
    /// Consumes the outer type, returning the inner type
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T, C> From<T> for Bytes<T, C> {
    fn from(value: T) -> Self {
        Self(value, PhantomData)
    }
}

impl<T: AsRef<U>, U: ?Sized, C> AsRef<U> for Bytes<T, C> {
    fn as_ref(&self) -> &U {
        self.0.as_ref()
    }
}

impl<T: AsMut<U>, U: ?Sized, C> AsMut<U> for Bytes<T, C> {
    fn as_mut(&mut self) -> &mut U {
        self.0.as_mut()
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
            let b64 = Cow::<'de, str>::deserialize(deserializer)?;
            let buf = base64::decode_config(b64.as_ref(), C::CONFIG)
                .map_err(|_| D::Error::custom("invalid base64"))?;
            Ok(Self(buf.into(), PhantomData))
        } else {
            Ok(Self(Vec::deserialize(deserializer)?.into(), PhantomData))
        }
    }
}

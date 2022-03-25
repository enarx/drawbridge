// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use core::ops::{Deref, DerefMut};
use core::str::FromStr;

use serde::{de::Error as _, Deserialize, Serialize};
use sha2::digest::{generic_array::GenericArray, OutputSizeUser};
use sha2::{Sha224, Sha256, Sha384, Sha512};

#[derive(Clone, Default)]
pub(super) struct Buffer<T: OutputSizeUser>(pub GenericArray<u8, T::OutputSize>);

impl<T: OutputSizeUser> Deref for Buffer<T> {
    type Target = GenericArray<u8, T::OutputSize>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: OutputSizeUser> DerefMut for Buffer<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: OutputSizeUser> Eq for Buffer<T> {}
impl<T: OutputSizeUser> PartialEq for Buffer<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(super) enum Inner {
    Sha224(Buffer<Sha224>),
    Sha256(Buffer<Sha256>),
    Sha384(Buffer<Sha384>),
    Sha512(Buffer<Sha512>),
}

impl Inner {
    fn name(&self) -> &'static str {
        match self {
            Self::Sha224(..) => "sha224",
            Self::Sha256(..) => "sha256",
            Self::Sha384(..) => "sha384",
            Self::Sha512(..) => "sha512",
        }
    }
}

impl AsRef<[u8]> for Inner {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::Sha224(b) => &*b,
            Self::Sha256(b) => &*b,
            Self::Sha384(b) => &*b,
            Self::Sha512(b) => &*b,
        }
    }
}

impl AsMut<[u8]> for Inner {
    fn as_mut(&mut self) -> &mut [u8] {
        match self {
            Self::Sha224(b) => &mut *b,
            Self::Sha256(b) => &mut *b,
            Self::Sha384(b) => &mut *b,
            Self::Sha512(b) => &mut *b,
        }
    }
}

impl std::fmt::Display for Inner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let b64 = base64::encode_config(self.as_ref(), base64::URL_SAFE_NO_PAD);
        write!(f, "{}:{}", self.name(), b64)
    }
}

impl FromStr for Inner {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let index = s.find(':').ok_or(Error::MissingColon)?;

        let (alg, b64) = s.split_at(index);
        let b64 = &b64[1..];

        let mut hash = match alg {
            "sha224" => Self::Sha224(Default::default()),
            "sha256" => Self::Sha256(Default::default()),
            "sha384" => Self::Sha384(Default::default()),
            "sha512" => Self::Sha512(Default::default()),
            _ => return Err(Error::UnknownAlgorithm),
        };

        if b64.len() != (hash.as_ref().len() * 8 + 5) / 6 {
            return Err(Error::InvalidLength);
        }

        base64::decode_config_slice(b64, base64::URL_SAFE_NO_PAD, hash.as_mut())?;

        Ok(hash)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    MissingColon,
    InvalidLength,
    UnknownAlgorithm,
    Decode(base64::DecodeError),
}

impl From<base64::DecodeError> for Error {
    fn from(value: base64::DecodeError) -> Self {
        Self::Decode(value)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Hash(pub(super) Inner);

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Hash {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Inner::from_str(s).map(Self)
    }
}

impl Serialize for Hash {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(|_| D::Error::custom("invalid hash"))
    }
}

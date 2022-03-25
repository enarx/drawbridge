// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use drawbridge_hash::{Error, Hash};
use drawbridge_http::http::{Request, StatusCode};
use drawbridge_http::{async_trait, FromRequest};

use serde::{de::Error as _, Deserialize, Serialize};

#[derive(Clone)]
pub struct Node {
    name: String,
    hash: Hash,
}

impl Eq for Node {}
impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl std::hash::Hash for Node {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl AsRef<str> for Node {
    fn as_ref(&self) -> &str {
        self.name.as_ref()
    }
}

impl Deref for Node {
    type Target = Hash;

    fn deref(&self) -> &Self::Target {
        &self.hash
    }
}

impl DerefMut for Node {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.hash
    }
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name.fmt(f)
    }
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Node").field(&self.name).finish()
    }
}

impl From<Hash> for Node {
    fn from(hash: Hash) -> Self {
        let name = format!("{}", hash);
        Self { name, hash }
    }
}

impl FromStr for Node {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hash = s.parse()?;
        let name = s.to_string();
        Ok(Node { name, hash })
    }
}

impl Serialize for Node {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.name.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        let hash = name.parse().map_err(|_| D::Error::custom("invalid hash"))?;
        Ok(Node { name, hash })
    }
}

#[async_trait]
impl FromRequest for Node {
    type Error = StatusCode;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error> {
        let etag = req.header("ETag").ok_or(StatusCode::BadRequest)?;
        etag.as_str().parse().or(Err(StatusCode::BadRequest))
    }
}

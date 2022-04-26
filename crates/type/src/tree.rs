// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use crate::digest::ContentDigest;

use std::collections::{BTreeMap, HashMap};
use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(feature = "axum")]
use axum::{
    body::HttpBody,
    extract::{FromRequest, RequestParts},
    http::StatusCode,
};

/// A directory entry
///
/// Note that this type is designed to be extensible. Therefore, the fields
/// here represent the minimum required fields. Other fields may be present.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// The hash of this entry
    pub digest: ContentDigest,

    /// Custom fields
    #[serde(flatten)]
    pub custom: HashMap<String, Value>,
}

impl Entry {
    pub const TYPE: &'static str = "application/vnd.drawbridge.entry.v1+json";
}

/// A directory
///
/// A directory is simply a sorted name to `Entry` map.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Directory(BTreeMap<String, Entry>);

impl Directory {
    pub const TYPE: &'static str = "application/vnd.drawbridge.directory.v1+json";
}

impl Deref for Directory {
    type Target = BTreeMap<String, Entry>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Directory {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Path(Vec<String>);

impl FromStr for Path {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[inline]
        fn valid(part: impl AsRef<str>) -> bool {
            let part = part.as_ref();
            !part.is_empty()
                && part
                    .find(|c| !matches!(c, '0'..='9' | 'a'..='z' | 'A'..='Z' | '-'))
                    .is_none()
        }

        let path = s.split_terminator('/').map(Into::into).collect::<Vec<_>>();
        if !path.iter().all(valid) {
            Err("Invalid path")
        } else {
            Ok(Self(path))
        }
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.join("/"))
    }
}

#[cfg(feature = "axum")]
#[axum::async_trait]
impl<B> FromRequest<B> for Path
where
    B: Send + HttpBody,
    B::Error: Sync + Send + std::error::Error + 'static,
    B::Data: Send,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        req.uri()
            .path()
            .strip_prefix('/')
            .expect("invalid URI")
            .parse()
            .map_err(|e| (StatusCode::BAD_REQUEST, e))
    }
}

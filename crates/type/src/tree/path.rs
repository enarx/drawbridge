// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

#[cfg(feature = "axum")]
use axum::{
    body::HttpBody,
    extract::{FromRequest, RequestParts},
    http::StatusCode,
};

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

impl Deref for Path {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Path {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
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

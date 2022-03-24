// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use drawbridge_http::http::{Request, StatusCode};
use drawbridge_http::{async_trait, FromRequest};

use semver::{Error, Version};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag(Version);

impl Deref for Tag {
    type Target = Version;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Tag {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromStr for Tag {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

#[async_trait]
impl FromRequest for Tag {
    type Error = StatusCode;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error> {
        req.url().path()[1..]
            .parse()
            .or(Err(StatusCode::BadRequest))
    }
}

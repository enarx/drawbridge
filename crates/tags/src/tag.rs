// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use std::hash::Hash;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use drawbridge_http::http::{self, Error, Request, StatusCode};
use drawbridge_http::{async_trait, FromRequest};

use semver::{self, Version};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    type Err = semver::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

#[async_trait]
impl FromRequest for Tag {
    async fn from_request(req: &mut Request) -> http::Result<Self> {
        req.url().path()[1..]
            .parse()
            .map_err(|_| Error::from_str(StatusCode::BadRequest, ""))
    }
}

#[cfg(test)]
mod test {
    use crate::Tag;

    #[test]
    fn parse_string() {
        let test_value: Tag = "1.2.3-beta".parse().unwrap();
        assert_eq!(test_value.major, 1);
        assert_eq!(test_value.minor, 2);
        assert_eq!(test_value.patch, 3);
        assert_eq!(test_value.pre.to_string(), "beta");
    }
}

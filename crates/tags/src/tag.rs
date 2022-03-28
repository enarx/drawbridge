// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use io::Error;
use std::hash::Hash;
use std::io;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use drawbridge_http::http::{Request, StatusCode};
use drawbridge_http::{async_trait, FromRequest};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tag(String);

impl Tag {
    pub const TYPE_ENTRY: &'static str = "application/vnd.drawbridge.tag.v1+json";
    pub const TYPE_JOSE: &'static str = "application/jose+json";
}

impl Deref for Tag {
    type Target = String;

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
        Ok(Tag(String::from(s)))
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

#[cfg(test)]
mod test {
    use crate::Tag;
    use std::str::FromStr;

    #[test]
    fn tag_test() {
        let test_string = "foo_bar_baz";
        let test_tag = Tag::from_str(test_string).unwrap();
        assert_eq!(test_string, test_tag);
    }
}

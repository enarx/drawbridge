use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use drawbridge_http::http::{Request, StatusCode};
use drawbridge_http::{async_trait, FromRequest};

use semver;
use serde::{Deserialize, Serialize};
use std::hash;

#[derive(Debug, Clone, hash::Hash)]
pub struct Tag(semver::Version);

impl Eq for Tag {}
impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Deref for Tag {
    type Target = semver::Version;

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

impl Serialize for Tag {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Tag {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        semver::Version::deserialize(deserializer).map(Self)
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
    fn parse_string() {
        let test_value = Tag::from_str("1.2.3-beta").unwrap();
        assert_eq!(test_value.major, 1);
        assert_eq!(test_value.minor, 2);
        assert_eq!(test_value.patch, 3);
        assert_eq!(test_value.pre.to_string(), "beta");
    }
}

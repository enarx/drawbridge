// SPDX-License-Identifier: Apache-2.0

use super::Path;

use std::fmt::Display;
use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::bail;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialOrd, PartialEq, Serialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Name(String);

impl Name {
    #[inline]
    fn validate(s: impl AsRef<str>) -> anyhow::Result<()> {
        let s = s.as_ref();
        if s.is_empty() {
            bail!("empty entry name")
        } else if s
            .find(|c| !matches!(c, '0'..='9' | 'a'..='z' | 'A'..='Z' | '-' | '_' | '.' | ':'))
            .is_some()
        {
            bail!("invalid characters in entry name")
        } else {
            Ok(())
        }
    }

    pub fn join(self, name: Name) -> Path {
        vec![self, name].into_iter().collect()
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<String> for Name {
    fn as_ref(&self) -> &String {
        &self.0
    }
}

impl Deref for Name {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Name {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        name.try_into().map_err(D::Error::custom)
    }
}

impl Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Name> for PathBuf {
    fn from(name: Name) -> Self {
        Self::from(name.0)
    }
}

impl From<Name> for String {
    fn from(name: Name) -> Self {
        name.0
    }
}

impl FromStr for Name {
    type Err = anyhow::Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::validate(s).map(|()| Self(s.into()))
    }
}

impl TryFrom<String> for Name {
    type Error = anyhow::Error;

    #[inline]
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::validate(&s).map(|()| Self(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str() {
        assert!("/".parse::<Name>().is_err());
        assert!("/test".parse::<Name>().is_err());
        assert!("test/".parse::<Name>().is_err());

        assert_eq!("foo".parse::<Name>().unwrap(), Name("foo".into()));
        assert_eq!("some.txt".parse::<Name>().unwrap(), Name("some.txt".into()));
        assert_eq!(
            "my_wasm.wasm".parse::<Name>().unwrap(),
            Name("my_wasm.wasm".into())
        );
        assert_eq!(
            "not.a.cor-Rec.t.eX.tens.si0n_".parse::<Name>().unwrap(),
            Name("not.a.cor-Rec.t.eX.tens.si0n_".into())
        );
    }
}

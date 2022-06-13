// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::Path;

use std::fmt::Display;
use std::ops::Deref;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialOrd, PartialEq, Serialize)]
pub struct Name(String);

impl Name {
    pub fn join(self, name: Name) -> Path {
        vec![self, name].into_iter().collect()
    }
}

impl FromStr for Name {
    type Err = &'static str;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty()
            || s.find(|c| !matches!(c, '0'..='9' | 'a'..='z' | 'A'..='Z' | '-' | '_' | '.'))
                .is_some()
        {
            Err("invalid name")
        } else {
            Ok(Self(s.into()))
        }
    }
}

impl Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        self
    }
}

impl Deref for Name {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
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

        assert_eq!("foo".parse(), Ok(Name("foo".into())));
        assert_eq!("some.txt".parse(), Ok(Name("some.txt".into())));
        assert_eq!("my_wasm.wasm".parse(), Ok(Name("my_wasm.wasm".into())));
        assert_eq!(
            "not.a.cor-Rec.t.eX.tens.si0n_".parse(),
            Ok(Name("not.a.cor-Rec.t.eX.tens.si0n_".into()))
        );
    }
}

// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::fmt::Display;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A user name
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[repr(transparent)]
pub struct Name(String);

impl FromStr for Name {
    type Err = &'static str;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Err("user name cannot be empty")
        } else if s
            .find(|c| !matches!(c, '0'..='9' | 'a'..='z' | 'A'..='Z'))
            .is_some()
        {
            Err("invalid user name")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str() {
        assert!("".parse::<Name>().is_err());
        assert!(" ".parse::<Name>().is_err());
        assert!("/".parse::<Name>().is_err());
        assert!("name/".parse::<Name>().is_err());
        assert!("/name".parse::<Name>().is_err());
        assert!("n%ame".parse::<Name>().is_err());
        assert!("n.ame".parse::<Name>().is_err());

        assert_eq!("name".parse(), Ok(Name("name".into())));
        assert_eq!("n4M3".parse(), Ok(Name("n4M3".into())));
    }
}

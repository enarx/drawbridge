// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::fmt::Display;
use std::str::FromStr;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

/// A user name
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[repr(transparent)]
pub struct Name(String);

impl FromStr for Name {
    type Err = anyhow::Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Err(anyhow!("empty user name"))
        } else if s
            .find(|c| !matches!(c, '0'..='9' | 'a'..='z' | 'A'..='Z'))
            .is_some()
        {
            Err(anyhow!("invalid characters in user name"))
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

        assert_eq!("name".parse::<Name>().unwrap(), Name("name".into()));
        assert_eq!("n4M3".parse::<Name>().unwrap(), Name("n4M3".into()));
    }
}

// SPDX-License-Identifier: Apache-2.0

use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Name(semver::Version);

impl FromStr for Name {
    type Err = semver::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Name)
    }
}

impl Deref for Name {
    type Target = semver::Version;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Name {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str() {
        for s in ["", "=", "/", "v1.2/3", "v1.2.3"] {
            assert!(
                s.parse::<Name>().is_err(),
                "input '{}' should fail to parse",
                s
            );
        }

        for (s, expected) in [
            (
                "1.2.3",
                semver::Version {
                    major: 1,
                    minor: 2,
                    patch: 3,
                    pre: Default::default(),
                    build: Default::default(),
                },
            ),
            (
                "1.2.3-test",
                semver::Version {
                    major: 1,
                    minor: 2,
                    patch: 3,
                    pre: semver::Prerelease::new("test").unwrap(),
                    build: Default::default(),
                },
            ),
        ] {
            assert_eq!(
                s.parse::<Name>().unwrap(),
                Name(expected),
                "input '{}' should succeed to parse",
                s
            );
        }
    }
}

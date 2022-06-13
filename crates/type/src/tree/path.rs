// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::Name;

use std::fmt::Display;
use std::ops::Deref;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Path(Vec<Name>);

impl Deref for Path {
    type Target = Vec<Name>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Path {
    pub const ROOT: Self = Self(vec![]);

    pub fn intersperse(&self, sep: &str) -> String {
        let mut it = self.0.iter();
        match it.next() {
            None => Default::default(),
            Some(first) => {
                let mut s = String::with_capacity(
                    self.0.iter().map(|p| p.len()).sum::<usize>() + self.0.len() - 1,
                );
                s.push_str(first);
                for p in it {
                    s.push_str(sep);
                    s.push_str(p);
                }
                s
            }
        }
    }
}

impl From<Name> for Path {
    fn from(name: Name) -> Self {
        Self(vec![name])
    }
}

impl FromIterator<Name> for Path {
    fn from_iter<T: IntoIterator<Item = Name>>(iter: T) -> Self {
        Self(Vec::<Name>::from_iter(iter))
    }
}

impl FromStr for Path {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim_start_matches('/')
            .split_terminator('/')
            .map(FromStr::from_str)
            .collect::<Result<Vec<_>, Self::Err>>()
            .map(Self)
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.intersperse("/"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str() {
        assert_eq!("/".parse::<Path>(), Ok(Path::ROOT));
        assert_eq!("/foo".parse(), Ok(Path(vec!["foo".parse().unwrap()])));
        assert_eq!("/foo/".parse(), Ok(Path(vec!["foo".parse().unwrap()])));
        assert_eq!(
            "/foo/bar".parse(),
            Ok(Path(vec!["foo".parse().unwrap(), "bar".parse().unwrap()]))
        );
    }
}

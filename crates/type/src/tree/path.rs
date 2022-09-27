// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::Name;

use std::fmt::Display;
use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Path(Vec<Name>);

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

impl AsRef<Vec<Name>> for Path {
    fn as_ref(&self) -> &Vec<Name> {
        &self.0
    }
}

impl AsRef<[Name]> for Path {
    fn as_ref(&self) -> &[Name] {
        &self.0
    }
}

impl Deref for Path {
    type Target = Vec<Name>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.intersperse("/"))
    }
}

impl From<Name> for Path {
    fn from(name: Name) -> Self {
        Self(vec![name])
    }
}

impl From<Path> for PathBuf {
    fn from(path: Path) -> Self {
        path.into_iter().map(PathBuf::from).collect()
    }
}

impl FromIterator<Name> for Path {
    fn from_iter<T: IntoIterator<Item = Name>>(iter: T) -> Self {
        Self(Vec::<Name>::from_iter(iter))
    }
}

impl FromStr for Path {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim_start_matches('/')
            .split_terminator('/')
            .map(FromStr::from_str)
            .collect::<Result<Vec<_>, Self::Err>>()
            .map(Self)
    }
}

impl IntoIterator for Path {
    type Item = Name;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str() {
        assert_eq!("/".parse::<Path>().unwrap(), Path::ROOT);
        assert_eq!(
            "/foo".parse::<Path>().unwrap(),
            Path(vec!["foo".parse().unwrap()])
        );
        assert_eq!(
            "/foo/".parse::<Path>().unwrap(),
            Path(vec!["foo".parse().unwrap()])
        );
        assert_eq!(
            "/foo/bar".parse::<Path>().unwrap(),
            Path(vec!["foo".parse().unwrap(), "bar".parse().unwrap()])
        );
    }
}

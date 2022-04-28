// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Path(Vec<String>);

impl FromStr for Path {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[inline]
        fn valid(part: impl AsRef<str>) -> bool {
            let part = part.as_ref();
            !part.is_empty()
                && part
                    .find(|c| !matches!(c, '0'..='9' | 'a'..='z' | 'A'..='Z' | '-' | '_'))
                    .is_none()
        }

        let path = s
            .trim_start_matches('/')
            .split_terminator('/')
            .map(Into::into)
            .collect::<Vec<_>>();
        if !path.iter().all(valid) {
            Err("Invalid path")
        } else {
            Ok(Self(path))
        }
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.join("/"))
    }
}

impl Deref for Path {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Path {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str() {
        assert_eq!("/".parse(), Ok(Path(Default::default())));
        assert_eq!("/foo".parse(), Ok(Path(vec!["foo".into()])));
        assert_eq!("/foo/".parse(), Ok(Path(vec!["foo".into()])));
        assert_eq!(
            "/foo/bar".parse(),
            Ok(Path(vec!["foo".into(), "bar".into()]))
        );
    }
}

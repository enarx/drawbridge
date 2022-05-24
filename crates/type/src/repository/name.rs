// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::fmt::Display;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A repository name
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Name {
    owner: String,
    groups: Vec<String>,
    name: String,
}

impl FromStr for Name {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[inline]
        fn valid(part: impl AsRef<str>) -> bool {
            let part = part.as_ref();
            !part.is_empty()
                && part
                    .find(|c| !matches!(c, '0'..='9' | 'a'..='z' | 'A'..='Z' | '-'))
                    .is_none()
        }

        let mut namespace = s.split_terminator('/').map(Into::into);
        let owner = namespace
            .next()
            .ok_or("Repository owner must be specified")?;
        let mut namespace = namespace.collect::<Vec<_>>();
        let name = namespace.pop().ok_or("Repository name must be specified")?;
        let groups = namespace;
        if !valid(&owner) || !valid(&name) || !groups.iter().all(valid) {
            Err("Invalid namespace")
        } else {
            Ok(Self {
                owner,
                groups,
                name,
            })
        }
    }
}

impl Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}/{}",
            self.owner,
            self.groups
                .iter()
                .fold("".into(), |acc, x| format!("{}/{}", acc, x)),
            self.name,
        )
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
        assert!("name".parse::<Name>().is_err());
        assert!("owner/".parse::<Name>().is_err());
        assert!("/name".parse::<Name>().is_err());
        assert!("owner//name".parse::<Name>().is_err());
        assert!("owner/group///name".parse::<Name>().is_err());
        assert!("owner/g%roup/name".parse::<Name>().is_err());
        assert!("owner/gяoup/name".parse::<Name>().is_err());
        assert!("owner /group/name".parse::<Name>().is_err());
        assert!("owner/gr☣up/name".parse::<Name>().is_err());
        assert!("o.wner/group/name".parse::<Name>().is_err());

        assert_eq!(
            "owner/name".parse(),
            Ok(Name {
                owner: "owner".into(),
                groups: vec![],
                name: "name".into(),
            })
        );
        assert_eq!(
            "owner/name/".parse(),
            Ok(Name {
                owner: "owner".into(),
                groups: vec![],
                name: "name".into(),
            })
        );
        assert_eq!(
            "owner/group/name".parse(),
            Ok(Name {
                owner: "owner".into(),
                groups: vec!["group".into()],
                name: "name".into(),
            })
        );
        assert_eq!(
            "owner/group/subgroup/name".parse(),
            Ok(Name {
                owner: "owner".into(),
                groups: vec!["group".into(), "subgroup".into()],
                name: "name".into(),
            })
        );
        assert_eq!(
            "0WnEr/gr0up/subgr0up/-n4mE".parse(),
            Ok(Name {
                owner: "0WnEr".into(),
                groups: vec!["gr0up".into(), "subgr0up".into()],
                name: "-n4mE".into(),
            })
        );
    }
}

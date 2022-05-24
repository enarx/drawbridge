// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

pub mod github;

use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Provider {
    GitHub,
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Provider::GitHub => "GitHub.com",
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Provider;

    #[test]
    fn auth_type_display() {
        assert_eq!(format!("{}", Provider::GitHub), "GitHub.com");
    }
}

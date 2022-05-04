// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use std::fmt;

#[derive(Debug)]
pub enum Error {
    Request(reqwest::Error),
    Serde(String),
    OAuth(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Error::Request(e) => format!("reqwest error: {}", e),
                Error::Serde(e) => format!("Serde error: {}", e),
                Error::OAuth(e) => format!("OAuth error: {}", e),
            }
        )
    }
}

impl std::error::Error for Error {}

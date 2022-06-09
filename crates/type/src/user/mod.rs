// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

mod config;
mod name;

pub use config::*;
pub use name::*;

use std::fmt::Display;
use std::str::FromStr;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Context {
    pub name: Name,
}

impl FromStr for Context {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let name = s.parse()?;
        Ok(Self { name })
    }
}

impl Display for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[cfg(feature = "axum")]
#[axum::async_trait]
impl<B: Send> axum::extract::FromRequest<B> for Context {
    type Rejection = axum::http::StatusCode;

    async fn from_request(
        req: &mut axum::extract::RequestParts<B>,
    ) -> Result<Self, Self::Rejection> {
        let axum::Extension(name) = req.extract().await.map_err(|e| {
            eprintln!(
                "{}",
                anyhow::Error::new(e).context("failed to extract user name")
            );
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;
        Ok(Self { name })
    }
}

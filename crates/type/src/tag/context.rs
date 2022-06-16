// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::RepositoryContext;
use super::Name;

use std::fmt::Display;
use std::str::FromStr;

use anyhow::{anyhow, Context as _};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Context {
    pub repository: RepositoryContext,
    pub name: Name,
}

impl FromStr for Context {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (repository, name) = s
            .split_once(':')
            .ok_or_else(|| anyhow!("`:` character not found"))?;
        let repository = repository
            .parse()
            .context("failed to parse repository name")?;
        let name = name.parse().context("failed to parse semantic version")?;
        Ok(Self { repository, name })
    }
}

impl Display for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.repository, self.name)
    }
}

#[cfg(feature = "axum")]
#[axum::async_trait]
impl<B: Send> axum::extract::FromRequest<B> for Context {
    type Rejection = (axum::http::StatusCode, String);

    async fn from_request(
        req: &mut axum::extract::RequestParts<B>,
    ) -> Result<Self, Self::Rejection> {
        let repository = req.extract().await?;
        let axum::Extension(name) = req.extract().await.map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                anyhow::Error::new(e)
                    .context("failed to extract tag context")
                    .to_string(),
            )
        })?;
        Ok(Self { repository, name })
    }
}

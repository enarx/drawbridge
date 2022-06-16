// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::UserContext;
use super::Name;

use std::fmt::Display;
use std::str::FromStr;

use anyhow::{anyhow, Context as _};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Context {
    pub owner: UserContext,
    pub name: Name,
}

impl FromStr for Context {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (owner, name) = s
            .split_once('/')
            .ok_or_else(|| anyhow!("`/` character not found"))?;
        let owner = owner.parse().context("failed to parse user context")?;
        let name = name.parse().context("failed to parse repository name")?;
        Ok(Self { owner, name })
    }
}

impl Display for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.owner, self.name)
    }
}

#[cfg(feature = "axum")]
#[axum::async_trait]
impl<B: Send> axum::extract::FromRequest<B> for Context {
    type Rejection = (axum::http::StatusCode, String);

    async fn from_request(
        req: &mut axum::extract::RequestParts<B>,
    ) -> Result<Self, Self::Rejection> {
        let owner = req.extract().await?;
        let axum::Extension(name) = req.extract().await.map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                anyhow::Error::new(e)
                    .context("failed to extract repository context")
                    .to_string(),
            )
        })?;
        Ok(Self { owner, name })
    }
}

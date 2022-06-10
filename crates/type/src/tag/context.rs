// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::RepositoryContext;
use super::Name;

use std::fmt::Display;
// TODO: Uncomment once compiler bug is fixed
//use std::str::FromStr;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Context {
    pub repository: RepositoryContext,
    pub name: Name,
}

// TODO: Uncomment once compiler bug is fixed
//impl FromStr for Context {
//    type Err = &'static str;
//
//    fn from_str(s: &str) -> Result<Self, Self::Err> {
//        let (repository, name) = s.split_once(":").ok_or("invalid tag context")?;
//        let repository = repository.parse()?;
//        let name = name.parse().map_err(Into::into)?;
//        Ok(Self { repository, name })
//    }
//}

impl Display for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.repository, self.name)
    }
}

#[cfg(feature = "axum")]
#[axum::async_trait]
impl<B: Send> axum::extract::FromRequest<B> for Context {
    type Rejection = axum::http::StatusCode;

    async fn from_request(
        req: &mut axum::extract::RequestParts<B>,
    ) -> Result<Self, Self::Rejection> {
        let repository = req.extract().await?;
        let axum::Extension(name) = req.extract().await.map_err(|e| {
            eprintln!(
                "{}",
                anyhow::Error::new(e).context("failed to extract tag name")
            );
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;
        Ok(Self { repository, name })
    }
}

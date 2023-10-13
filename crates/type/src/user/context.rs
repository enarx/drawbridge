// SPDX-License-Identifier: Apache-2.0

use super::Name;

use std::fmt::Display;
use std::str::FromStr;

use anyhow::Context as _;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Context {
    pub name: Name,
}

impl FromStr for Context {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let name = s.parse().context("failed to parse user name")?;
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
    type Rejection = (axum::http::StatusCode, String);

    async fn from_request(
        req: &mut axum::extract::RequestParts<B>,
    ) -> Result<Self, Self::Rejection> {
        let axum::Extension(name) = req.extract().await.map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                anyhow::Error::new(e)
                    .context("failed to extract user context")
                    .to_string(),
            )
        })?;
        Ok(Self { name })
    }
}

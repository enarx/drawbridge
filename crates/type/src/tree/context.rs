// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::TagContext;
use super::Path;

use std::fmt::Display;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Context {
    pub tag: TagContext,
    pub path: Path,
}

impl Display for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.tag, self.path)
    }
}

#[cfg(feature = "axum")]
#[axum::async_trait]
impl<B: Send> axum::extract::FromRequest<B> for Context {
    type Rejection = (axum::http::StatusCode, String);

    async fn from_request(
        req: &mut axum::extract::RequestParts<B>,
    ) -> Result<Self, Self::Rejection> {
        let tag = req.extract().await?;
        let axum::Extension(path) = req.extract().await.map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                anyhow::Error::new(e)
                    .context("failed to extract tree context")
                    .to_string(),
            )
        })?;
        Ok(Self { tag, path })
    }
}

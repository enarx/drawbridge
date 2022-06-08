// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{Client, Result, Tag};

use drawbridge_type::digest::Algorithms;
use drawbridge_type::{RepositoryConfig, RepositoryName, TagName};

use anyhow::{bail, Context};
use http::header::CONTENT_TYPE;
use http::StatusCode;
use mime::APPLICATION_JSON;

pub struct Repository<'a> {
    pub(crate) client: &'a Client,
    pub(crate) name: &'a RepositoryName,
}

impl Repository<'_> {
    pub fn tag<'a>(&'a self, name: &'a TagName) -> Tag<'_> {
        Tag { repo: self, name }
    }

    pub fn tags(&self) -> Result<Vec<TagName>> {
        let res = self
            .client
            .inner
            .get(
                self.client
                    .url
                    .join(&format!("{}/_tag", self.name))?
                    .as_str(),
            )
            .call()?;
        // TODO: Verify Content-Digest
        // TODO: Verify Content-Length
        // https://github.com/profianinc/drawbridge/issues/103
        match StatusCode::from_u16(res.status()) {
            Ok(StatusCode::OK) => res.into_json().map_err(Into::into),
            _ => bail!("unexpected status code: {}", res.status()),
        }
    }

    pub fn create(&self, conf: &RepositoryConfig) -> Result<bool> {
        let body =
            serde_json::to_vec(&conf).context("failed to encode repository config to JSON")?;
        let content_digest = Algorithms::default()
            .read_sync(&body[..])
            .context("failed to compute repository config digest")?
            .to_string();

        let res = self
            .client
            .inner
            .put(self.client.url.join(&self.name.to_string())?.as_str())
            .set(CONTENT_TYPE.as_str(), APPLICATION_JSON.as_ref())
            .set("Content-Digest", &content_digest)
            .send_bytes(&body)?;
        match StatusCode::from_u16(res.status()) {
            Ok(StatusCode::CREATED) => Ok(true),
            Ok(StatusCode::OK) => Ok(false),
            _ => bail!("unexpected status code: {}", res.status()),
        }
    }

    pub fn get(&self) -> Result<RepositoryConfig> {
        let res = self
            .client
            .inner
            .get(self.client.url.join(&self.name.to_string())?.as_str())
            .call()?;
        // TODO: Verify Content-Digest
        // TODO: Verify Content-Length
        // https://github.com/profianinc/drawbridge/issues/103
        match StatusCode::from_u16(res.status()) {
            Ok(StatusCode::OK) => res.into_json().map_err(Into::into),
            _ => bail!("unexpected status code: {}", res.status()),
        }
    }
}

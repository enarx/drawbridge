// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::{Client, Result, Tag};

use drawbridge_type::{RepositoryConfig, RepositoryName, TagName};

use anyhow::bail;
use http::StatusCode;

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
        let res = self
            .client
            .inner
            .put(self.client.url.join(&self.name.to_string())?.as_str())
            // TODO: Calculate and set Content-Digest
            // https://github.com/profianinc/drawbridge/issues/102
            .send_json(conf)?;
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

// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::{Client, Tag};

use std::error::Error;

use drawbridge_type::{RepositoryConfig, RepositoryName, TagName};

use reqwest::StatusCode;

pub struct Repository<'a> {
    pub(crate) client: &'a Client,
    pub(crate) name: &'a RepositoryName,
}

impl Repository<'_> {
    pub fn tag<'a>(&'a self, name: &'a TagName) -> Tag<'_> {
        Tag { repo: self, name }
    }

    pub fn tags(&self) -> Result<Vec<TagName>, Box<dyn Error>> {
        let res = self
            .client
            .inner
            .get(self.client.url.join(&format!("{}/_tag", self.name))?)
            .send()?
            .error_for_status()?;
        // TODO: Verify Content-Digest
        // TODO: Verify Content-Length
        // https://github.com/profianinc/drawbridge/issues/103
        match res.status() {
            StatusCode::OK => res.json().map_err(Into::into),
            _ => Err("unexpected status code")?,
        }
    }

    pub fn create(&self, conf: &RepositoryConfig) -> Result<bool, Box<dyn Error>> {
        let res = self
            .client
            .inner
            .put(self.client.url.join(&self.name.to_string())?)
            // TODO: Calculate and set Content-Digest
            .json(conf)
            .send()?
            .error_for_status()?;
        match res.status() {
            StatusCode::CREATED => Ok(true),
            StatusCode::OK => Ok(false),
            _ => Err("unexpected status code")?,
        }
    }

    pub fn get(&self) -> Result<RepositoryConfig, Box<dyn Error>> {
        let res = self
            .client
            .inner
            .get(self.client.url.join(&self.name.to_string())?)
            .send()?
            .error_for_status()?;
        // TODO: Verify Content-Digest
        // TODO: Verify Content-Length
        // https://github.com/profianinc/drawbridge/issues/103
        match res.status() {
            StatusCode::OK => res.json().map_err(Into::into),
            _ => Err("unexpected status code")?,
        }
    }
}

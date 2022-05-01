// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::{Node, Repository, Result};

use drawbridge_jose::jws::Jws;
use drawbridge_jose::MediaTyped;
use drawbridge_type::{TagEntry, TagName, TreeEntry, TreePath};

use anyhow::bail;
use reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE};
use reqwest::StatusCode;

pub struct Tag<'a> {
    pub(crate) repo: &'a Repository<'a>,
    pub(crate) name: &'a TagName,
}

impl Tag<'_> {
    pub fn path<'a>(&'a self, path: &'a TreePath) -> Node<'_> {
        Node { tag: self, path }
    }

    pub fn create(&self, entry: &TagEntry) -> Result<bool> {
        let body = serde_json::to_vec(&entry)?;
        let res = self
            .repo
            .client
            .inner
            .put(
                self.repo
                    .client
                    .url
                    .join(&format!("{}/_tag/{}", self.repo.name, self.name))?,
            )
            .header(
                CONTENT_TYPE,
                match entry {
                    TagEntry::Unsigned(..) => TreeEntry::TYPE,
                    TagEntry::Signed(..) => Jws::TYPE,
                },
            )
            .header(CONTENT_LENGTH, body.len()) // TODO: Verify if this is handled by reqwest
            // TODO: Calculate and set Content-Digest
            // https://github.com/profianinc/drawbridge/issues/102
            .body(body)
            .send()?
            .error_for_status()?;
        match res.status() {
            StatusCode::CREATED => Ok(true),
            StatusCode::OK => Ok(false),
            _ => bail!("unexpected status code: {}", res.status()),
        }
    }

    pub fn get(&self) -> Result<TagEntry> {
        let res = self
            .repo
            .client
            .inner
            .get(
                self.repo
                    .client
                    .url
                    .join(&format!("{}/_tag/{}", self.repo.name, self.name))?,
            )
            .send()?
            .error_for_status()?;
        // TODO: Verify Content-Digest
        // TODO: Verify Content-Length
        // https://github.com/profianinc/drawbridge/issues/103
        match res.status() {
            StatusCode::OK => res.json().map_err(Into::into),
            _ => bail!("unexpected status code: {}", res.status()),
        }
    }
}

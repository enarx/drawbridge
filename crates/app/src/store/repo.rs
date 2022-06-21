// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{CreateError, Entity, GetError, Tag};

use std::ops::Deref;

use drawbridge_type::digest::{Algorithms, ContentDigest};
use drawbridge_type::{Meta, TagEntry, TagName};

use anyhow::{anyhow, Context};
use camino::{Utf8Path, Utf8PathBuf};

#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct Repository<'a, P = Utf8PathBuf>(Entity<'a, P>);

impl<'a, P> Deref for Repository<'a, P> {
    type Target = Entity<'a, P>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, P> From<Entity<'a, P>> for Repository<'a, P> {
    fn from(entity: Entity<'a, P>) -> Self {
        Self(entity)
    }
}

impl<'a, P: AsRef<Utf8Path>> Repository<'a, P> {
    pub async fn tags(&self) -> Result<Vec<TagName>, GetError<anyhow::Error>> {
        self.read_dir("tags")
            .await?
            .try_fold(vec![], |mut names, entry| {
                let name = entry?
                    .file_name()
                    .context("failed to read tag name")?
                    .parse()
                    .context("failed to parse tag name")?;
                names.push(name);
                Ok(names)
            })
            .map_err(GetError::Internal)
    }

    pub async fn tags_json(&self) -> Result<(ContentDigest, Vec<u8>), GetError<anyhow::Error>> {
        // TODO: Optimize hash computation
        let tags = self.tags().await?;
        let buf = serde_json::to_vec(&tags)
            .context("failed to encode tags as JSON")
            .map_err(GetError::Internal)?;
        let (n, hash) = Algorithms::default()
            .read_sync(&buf[..])
            .context("failed to compute tag digest")
            .map_err(GetError::Internal)?;
        if n != buf.len() as u64 {
            return Err(GetError::Internal(anyhow!(
                "invalid amount of bytes read, expected: {}, got {n}",
                buf.len(),
            )));
        }
        Ok((hash, buf))
    }

    pub fn tag(&self, name: &TagName) -> Tag<'a, Utf8PathBuf> {
        self.child(format!("tags/{name}")).into()
    }

    pub async fn create_tag(
        &self,
        name: &TagName,
        meta: Meta,
        entry: &TagEntry,
    ) -> Result<Tag<'a, Utf8PathBuf>, CreateError<anyhow::Error>> {
        let tag = self.tag(name);
        tag.create_dir("").await?;
        tag.create_json(meta, entry).await?;
        Ok(tag)
    }
}

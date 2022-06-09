// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{CreateError, Entity, GetError, Tag};

use std::borrow::Borrow;
use std::ops::Deref;

use drawbridge_type::{Meta, RepositoryConfig, RepositoryName, TagName};

use anyhow::Context;
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

impl<'a> Repository<'a, Utf8PathBuf> {
    pub fn new(
        entity: Entity<'a, impl AsRef<Utf8Path>>,
        name: impl Borrow<RepositoryName>,
    ) -> Self {
        Self(entity.child(name.borrow().to_string()))
    }
}

impl<'a, P: AsRef<Utf8Path>> Repository<'a, P> {
    pub async fn create(
        &self,
        meta: Meta,
        conf: &RepositoryConfig,
    ) -> Result<(), CreateError<anyhow::Error>> {
        self.0.create_json(meta, conf).await?;
        self.0.create_dir("tags").await
    }

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

    pub fn tag(&self, name: &TagName) -> Tag<'a, Utf8PathBuf> {
        Tag::new(self.child("tags"), name)
    }
}

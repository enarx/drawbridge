// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{dir_builder_defaults, CreateError, Entity, GetError, Tag};

use std::ops::Deref;

use drawbridge_type::{Meta, RepositoryConfig, RepositoryName, TagName};

use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use cap_async_std::fs_utf8::DirBuilder;

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
    pub fn new(entity: Entity<'a, impl AsRef<Utf8Path>>, name: &RepositoryName) -> Self {
        Self(entity.child(name.to_string()))
    }
}

impl<'a, P: AsRef<Utf8Path>> Repository<'a, P> {
    pub async fn create(
        &self,
        meta: Meta,
        conf: &RepositoryConfig,
    ) -> Result<(), CreateError<anyhow::Error>> {
        self.0
            .create_json_with(
                meta,
                conf,
                dir_builder_defaults(&mut DirBuilder::new()).recursive(true),
            )
            .await?;
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

    pub fn tag(&self, name: &TagName) -> Tag<'_, Utf8PathBuf> {
        Tag::new(self.child("tags"), name)
    }
}

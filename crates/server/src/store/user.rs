// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::{CreateError, Entity, Repository};

use std::ops::Deref;

use drawbridge_type::{Meta, RepositoryConfig, RepositoryName};

use camino::{Utf8Path, Utf8PathBuf};
use futures::try_join;

#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct User<'a, P = Utf8PathBuf>(Entity<'a, P>);

impl<'a, P> Deref for User<'a, P> {
    type Target = Entity<'a, P>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, P> From<Entity<'a, P>> for User<'a, P> {
    fn from(entity: Entity<'a, P>) -> Self {
        Self(entity)
    }
}

impl<'a, P: AsRef<Utf8Path>> User<'a, P> {
    pub fn repository(&self, name: &RepositoryName) -> Repository<'a, Utf8PathBuf> {
        self.0.child(format!("repos/{name}")).into()
    }

    pub async fn create_repository(
        &self,
        name: &RepositoryName,
        meta: Meta,
        conf: &RepositoryConfig,
    ) -> Result<Repository<'a, Utf8PathBuf>, CreateError<anyhow::Error>> {
        let repo = self.repository(name);
        repo.create_dir("").await?;
        try_join!(repo.create_json(meta, conf), repo.create_dir("tags"))?;
        Ok(repo)
    }
}

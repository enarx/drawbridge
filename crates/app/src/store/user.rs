// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{CreateError, Entity, Repository};

use std::borrow::Borrow;
use std::ops::Deref;

use drawbridge_type::{Meta, RepositoryName, UserConfig, UserName};

use camino::{Utf8Path, Utf8PathBuf};

#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct User<'a, P = Utf8PathBuf>(Entity<'a, P>);

impl<'a, P> Deref for User<'a, P> {
    type Target = Entity<'a, P>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> User<'a, Utf8PathBuf> {
    pub fn new(entity: Entity<'a, impl AsRef<Utf8Path>>, name: impl Borrow<UserName>) -> Self {
        Self(entity.child(name.borrow().to_string()))
    }
}

impl<'a, P: AsRef<Utf8Path>> User<'a, P> {
    pub async fn create(
        &self,
        meta: Meta,
        conf: &UserConfig,
    ) -> Result<(), CreateError<anyhow::Error>> {
        self.0.create_json(meta, conf).await?;
        self.0.create_dir("repos").await
    }

    pub fn repository(&self, name: &RepositoryName) -> Repository<'a> {
        Repository::new(self.0.child("repos"), name)
    }
}

// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{Entity, Repository, Result};

use std::ops::Deref;

use drawbridge_type::{RepositoryName, UserConfig, UserName};

use mime::APPLICATION_JSON;

#[repr(transparent)]
pub struct User<'a>(Entity<'a>);

impl<'a> Deref for User<'a> {
    type Target = Entity<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> User<'a> {
    pub fn new(entity: Entity<'a>, name: &UserName) -> Self {
        User(entity.child(&name.to_string()))
    }

    pub fn create(&self, conf: &UserConfig) -> Result<bool> {
        self.0.create_json(&APPLICATION_JSON, conf)
    }

    pub fn get(&self) -> Result<UserConfig> {
        self.0.get_json()
    }

    pub fn repository(&self, name: &RepositoryName) -> Repository<'a> {
        Repository::new(self.0.clone(), name)
    }
}

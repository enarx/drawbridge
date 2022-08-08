// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{scope, Entity, Repository, Result, Scope};

use std::ops::Deref;

use drawbridge_type::{RepositoryName, UserName, UserRecord};

use mime::APPLICATION_JSON;

#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct User<'a, S: Scope>(Entity<'a, S, scope::User>);

impl<'a, S: Scope> Deref for User<'a, S> {
    type Target = Entity<'a, S, scope::User>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, S: Scope> User<'a, S> {
    pub fn new(entity: Entity<'a, S, scope::Root>, name: &UserName) -> Self {
        User(entity.child(&name.to_string()))
    }

    pub fn create(&self, conf: &UserRecord) -> Result<bool> {
        self.0.create_json(&APPLICATION_JSON, conf)
    }

    pub fn get(&self) -> Result<UserRecord> {
        // TODO: Use a reasonable byte limit
        self.0.get_json(u64::MAX).map(|(_, v)| v)
    }

    pub fn repository(&self, name: &RepositoryName) -> Repository<'a, S> {
        Repository::new(self.0.clone(), name)
    }
}

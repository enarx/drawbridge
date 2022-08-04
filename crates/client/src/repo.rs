// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{scope, Entity, Result, Scope, Tag};

use std::ops::Deref;

use drawbridge_type::{RepositoryConfig, RepositoryName, TagName};

use mime::APPLICATION_JSON;

pub struct Repository<'a, S: Scope>(Entity<'a, S, scope::Repository>);

impl<'a, S: Scope> Deref for Repository<'a, S> {
    type Target = Entity<'a, S, scope::Repository>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, S: Scope> Repository<'a, S> {
    pub fn new(entity: Entity<'a, S, scope::User>, name: &RepositoryName) -> Repository<'a, S> {
        Repository(entity.child(&name.to_string()))
    }

    pub fn create(&self, conf: &RepositoryConfig) -> Result<bool> {
        self.0.create_json(&APPLICATION_JSON, conf)
    }

    pub fn get(&self) -> Result<RepositoryConfig> {
        // TODO: Use a reasonable byte limit
        self.0.get_json(u64::MAX).map(|(_, v)| v)
    }

    pub fn tags(&self) -> Result<Vec<TagName>> {
        self.0
            .child::<scope::Unknown>("_tag")
            .get_json(u64::MAX)
            .map(|(_, v)| v)
    }

    pub fn tag(&self, name: &TagName) -> Tag<'a, S> {
        Tag::new(self.child("_tag"), name)
    }
}

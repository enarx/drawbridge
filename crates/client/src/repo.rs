// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{Entity, Result, Tag};

use std::ops::Deref;

use drawbridge_type::{RepositoryConfig, RepositoryName, TagName};

use mime::APPLICATION_JSON;

pub struct Repository<'a>(Entity<'a>);

impl<'a> Deref for Repository<'a> {
    type Target = Entity<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> Repository<'a> {
    pub fn new(entity: Entity<'a>, name: &RepositoryName) -> Repository<'a> {
        Repository(entity.child(&name.to_string()))
    }

    pub fn create(&self, conf: &RepositoryConfig) -> Result<bool> {
        self.0.create_json(&APPLICATION_JSON, conf)
    }

    pub fn get(&self) -> Result<RepositoryConfig> {
        self.0.get_json()
    }

    pub fn tags(&self) -> Result<Vec<TagName>> {
        self.0.child("_tag").get_json()
    }

    pub fn tag(&self, name: &TagName) -> Tag<'a> {
        Tag::new(self.child("_tag"), name)
    }
}

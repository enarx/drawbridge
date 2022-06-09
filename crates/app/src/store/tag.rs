// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{CreateError, Entity, Node};

use std::borrow::Borrow;
use std::ops::Deref;

use drawbridge_type::{Meta, TagEntry, TagName, TreePath};

use camino::{Utf8Path, Utf8PathBuf};

#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct Tag<'a, P = Utf8PathBuf>(Entity<'a, P>);

impl<'a, P> Deref for Tag<'a, P> {
    type Target = Entity<'a, P>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> Tag<'a, Utf8PathBuf> {
    pub fn new(entity: Entity<'a, impl AsRef<Utf8Path>>, name: impl Borrow<TagName>) -> Self {
        Self(entity.child(name.borrow().to_string()))
    }
}

impl<'a, P: AsRef<Utf8Path>> Tag<'a, P> {
    pub async fn create(
        &self,
        meta: Meta,
        entry: &TagEntry,
    ) -> Result<(), CreateError<anyhow::Error>> {
        self.0.create_json(meta, entry).await
    }

    pub fn path(&self, path: &TreePath) -> Node<'a, Utf8PathBuf> {
        Node::new(self.0.child("tree"), path)
    }
}

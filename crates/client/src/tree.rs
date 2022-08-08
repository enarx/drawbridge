// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{scope, Entity, Result, Scope};

use std::io::Read;
use std::ops::Deref;

use drawbridge_type::{Meta, TreeDirectory, TreeEntry, TreePath};

use mime::Mime;
use ureq::serde::Serialize;

#[derive(Clone, Debug)]
pub struct Node<'a, S: Scope>(Entity<'a, S, scope::Node>);

impl<'a, S: Scope> Deref for Node<'a, S> {
    type Target = Entity<'a, S, scope::Node>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, S: Scope> Node<'a, S> {
    pub fn new(entity: Entity<'a, S, scope::Node>, path: &TreePath) -> Self {
        if path.is_empty() {
            Self(entity)
        } else {
            Self(entity.child(&path.to_string()))
        }
    }

    pub fn create_bytes(&self, mime: &Mime, data: impl AsRef<[u8]>) -> Result<bool> {
        self.0.create_bytes(mime, data)
    }

    pub fn create_json(&self, mime: &Mime, val: &impl Serialize) -> Result<bool> {
        self.0.create_json(mime, val)
    }

    pub fn create_from(&self, meta: &Meta, rdr: impl Read) -> Result<bool> {
        self.0.create_from(meta, rdr)
    }

    pub fn create_directory<C>(&self, dir: &TreeDirectory<TreeEntry<C>>) -> Result<bool> {
        let mime = TreeDirectory::<C>::TYPE
            .parse()
            .expect("failed to parse tree directory media type");
        self.create_json(&mime, dir)
    }
}

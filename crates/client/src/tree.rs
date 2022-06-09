// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{Entity, Result};

use std::io::Read;
use std::ops::Deref;

use drawbridge_type::{Meta, TreeDirectory, TreePath};

use mime::Mime;
use ureq::serde::Serialize;

pub struct Node<'a>(Entity<'a>);

impl<'a> Deref for Node<'a> {
    type Target = Entity<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> Node<'a> {
    pub fn new(entity: Entity<'a>, name: &TreePath) -> Self {
        Node(entity.child(&name.to_string()))
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

    pub fn create_directory(&self, dir: &TreeDirectory) -> Result<bool> {
        let mime = TreeDirectory::TYPE
            .parse()
            .expect("failed to parse tree directory media type");
        self.create_json(&mime, dir)
    }
}

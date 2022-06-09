// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{Entity, Node, Result};

use std::ops::Deref;

use drawbridge_jose::jws::Jws;
use drawbridge_jose::MediaTyped;
use drawbridge_type::{TagEntry, TagName, TreeEntry, TreePath};

pub struct Tag<'a>(Entity<'a>);

impl<'a> Deref for Tag<'a> {
    type Target = Entity<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> Tag<'a> {
    pub fn new(entity: Entity<'a>, name: &TagName) -> Self {
        Tag(entity.child(&name.to_string()))
    }

    pub fn create(&self, entry: &TagEntry) -> Result<bool> {
        let mime = match entry {
            TagEntry::Unsigned(..) => TreeEntry::TYPE,
            TagEntry::Signed(..) => Jws::TYPE,
        }
        .parse()
        .expect("failed to parse tag entry media type");
        self.0.create_json(&mime, entry)
    }

    pub fn get(&self) -> Result<TagEntry> {
        self.0.get_json()
    }

    pub fn path(&self, name: &TreePath) -> Node<'a> {
        Node::new(self.child("tree"), name)
    }
}

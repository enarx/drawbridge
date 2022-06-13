// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{Entity, Node, Result};

use std::collections::BTreeMap;
use std::ops::Deref;
use std::path::Path;

use drawbridge_jose::jws::Jws;
use drawbridge_jose::MediaTyped;
use drawbridge_type::TreeContent::{Directory, File};
use drawbridge_type::{TagEntry, TagName, Tree, TreeEntry, TreePath};

use ureq::serde::Serialize;

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

    pub fn create(&self, entry: &TagEntry<impl Serialize>) -> Result<bool> {
        let mime = match entry {
            TagEntry::Unsigned(..) => TreeEntry::<()>::TYPE,
            TagEntry::Signed(..) => Jws::TYPE,
        }
        .parse()
        .expect("failed to parse tag entry media type");
        self.0.create_json(&mime, entry)
    }

    // TODO: Support signed tags
    pub fn create_from_path_unsigned(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(bool, BTreeMap<TreePath, bool>)> {
        let tree = Tree::from_path_sync(path)?;
        let tag_created = self.create(&TagEntry::Unsigned(tree.root()))?;
        let tree_created = tree
            .into_iter()
            .map(
                |(
                    path,
                    TreeEntry {
                        ref meta,
                        ref content,
                        ..
                    },
                )| {
                    let node = Node::new(self.child("tree"), &path);
                    let created = match content {
                        File(file) => node.create_from(meta, file)?,
                        Directory(buf) => node.create_from(meta, buf.as_slice())?,
                    };
                    Ok((path, created))
                },
            )
            .collect::<Result<_>>()?;
        Ok((tag_created, tree_created))
    }

    pub fn get(&self) -> Result<TagEntry> {
        self.0.get_json()
    }

    pub fn path(&self, path: &TreePath) -> Node<'a> {
        Node::new(self.child("tree"), path)
    }
}

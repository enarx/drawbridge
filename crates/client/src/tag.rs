// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{scope, Entity, Node, Result, Scope};

use std::collections::BTreeMap;
use std::io::Seek;
use std::ops::Deref;
use std::path::Path;

use drawbridge_jose::jws::Jws;
use drawbridge_jose::MediaTyped;
use drawbridge_type::TreeContent::{Directory, File};
use drawbridge_type::{TagEntry, TagName, Tree, TreeEntry, TreePath};

use anyhow::Context;
use ureq::serde::Serialize;

#[derive(Clone, Debug)]
pub struct Tag<'a, S: Scope>(Entity<'a, S, scope::Tag>);

impl<'a, S: Scope> Deref for Tag<'a, S> {
    type Target = Entity<'a, S, scope::Tag>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, S: Scope> Tag<'a, S> {
    pub fn new(entity: Entity<'a, S, scope::Repository>, name: &TagName) -> Self {
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
            .map(|(path, TreeEntry { meta, content, .. })| {
                let node = Node::new(self.child("tree"), &path);
                let created = match content {
                    File(mut file) => {
                        file.rewind().context("failed to rewind file")?;
                        node.create_from(&meta, file)?
                    }
                    Directory(buf) => node.create_from(&meta, buf.as_slice())?,
                };
                Ok((path, created))
            })
            .collect::<Result<_>>()?;
        Ok((tag_created, tree_created))
    }

    pub fn get(&self) -> Result<TagEntry> {
        // TODO: Validate MIME type
        // TODO: Use a reasonable byte limit
        self.0.get_json(u64::MAX).map(|(_, v)| v)
    }

    pub fn path(&self, path: &TreePath) -> Node<'a, S> {
        Node::new(self.child("tree"), path)
    }
}

// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::{CreateError, Entity, Node};

use std::ops::Deref;

use drawbridge_type::{Meta, TreeDirectory, TreeEntry, TreePath};

use camino::{Utf8Path, Utf8PathBuf};
use futures::{try_join, AsyncRead};
use tracing::debug;

#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct Tag<'a, P = Utf8PathBuf>(Entity<'a, P>);

impl<'a, P> Deref for Tag<'a, P> {
    type Target = Entity<'a, P>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, P> From<Entity<'a, P>> for Tag<'a, P> {
    fn from(entity: Entity<'a, P>) -> Self {
        Self(entity)
    }
}

impl<'a, P: AsRef<Utf8Path>> Tag<'a, P> {
    pub fn node(&self, path: &TreePath) -> Node<'a, Utf8PathBuf> {
        if path.is_empty() {
            self.0.child("tree").into()
        } else {
            self.0
                .child(format!("tree/entries/{}", path.intersperse("/entries/")))
                .into()
        }
    }

    pub async fn create_file_node(
        &self,
        path: &TreePath,
        meta: Meta,
        rdr: impl Unpin + AsyncRead,
    ) -> Result<Node<'a, Utf8PathBuf>, CreateError<anyhow::Error>> {
        // TODO: Validate node hash against parents' expected values
        // https://github.com/profianinc/drawbridge/issues/77
        let node = self.node(path);
        node.create_dir("").await.map_err(|e| {
            debug!(target: "app::store::Tag::create_file_node", "failed to create content directory: {:?}", e);
            e
        })?;
        node.create_from_reader(meta, rdr).await?;
        Ok(node)
    }

    pub async fn create_directory_node(
        &self,
        path: &TreePath,
        meta: Meta,
        dir: &TreeDirectory<TreeEntry>,
    ) -> Result<Node<'a, Utf8PathBuf>, CreateError<anyhow::Error>> {
        // TODO: Validate node hash against parents' expected values
        // https://github.com/profianinc/drawbridge/issues/77
        let node = self.node(path);
        node.create_dir("").await.map_err(|e| {
            debug!(target: "app::store::Tag::create_directory_node", "failed to create content directory: {:?}", e);
            e
        })?;
        try_join!(node.create_json(meta, dir), node.create_dir("entries"))?;
        Ok(node)
    }
}

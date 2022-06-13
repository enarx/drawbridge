// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{CreateError, Entity};

use std::borrow::Borrow;
use std::ops::Deref;

use drawbridge_type::{Meta, TreeDirectory, TreeEntry, TreePath};

use camino::{Utf8Path, Utf8PathBuf};
use futures::AsyncRead;

#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct Node<'a, P = Utf8PathBuf>(Entity<'a, P>);

impl<'a, P> Deref for Node<'a, P> {
    type Target = Entity<'a, P>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> Node<'a, Utf8PathBuf> {
    pub fn new(entity: Entity<'a, impl AsRef<Utf8Path>>, path: impl Borrow<TreePath>) -> Self {
        let path = path.borrow();
        if path.is_empty() {
            Self(entity.child(""))
        } else {
            Self(entity.child(format!("entries/{}", path.intersperse("/entries/"))))
        }
    }
}

impl<'a, P: AsRef<Utf8Path>> Node<'a, P> {
    pub async fn create_file(
        &self,
        meta: Meta,
        rdr: impl Unpin + AsyncRead,
    ) -> Result<(), CreateError<anyhow::Error>> {
        // TODO: Validate node hash against parents' expected values
        // https://github.com/profianinc/drawbridge/issues/77
        self.0.create_from_reader(meta, rdr).await
    }

    pub async fn create_directory(
        &self,
        meta: Meta,
        dir: &TreeDirectory<TreeEntry>,
    ) -> Result<(), CreateError<anyhow::Error>> {
        // TODO: Validate node hash against parents' expected values
        // https://github.com/profianinc/drawbridge/issues/77
        self.0.create_json(meta, dir).await?;
        self.0.create_dir("entries").await
    }
}

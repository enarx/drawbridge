// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

mod entity;
mod repo;
mod tag;
mod tree;
mod user;

pub use entity::*;
pub use repo::*;
pub use tag::*;
pub use tree::*;
pub use user::*;

use drawbridge_type::{Meta, RepositoryContext, TagContext, TreeContext, UserContext, UserRecord};

use async_std::io;
use camino::{Utf8Path, Utf8PathBuf};
use cap_async_std::fs_utf8::Dir;
use futures::try_join;

#[derive(Debug)]
pub struct Store {
    root: Dir,
}

async fn upsert_dir(root: &Dir, path: impl AsRef<Utf8Path>) -> io::Result<()> {
    let path = path.as_ref();
    if !root.is_dir(path).await {
        root.create_dir(path)
    } else {
        Ok(())
    }
}

impl Store {
    /// Initalizes a new [Store] at `root`
    pub async fn new(root: Dir) -> io::Result<Self> {
        upsert_dir(&root, "users").await?;
        Ok(Self { root })
    }

    pub fn user(&self, UserContext { name }: &UserContext) -> User<'_, Utf8PathBuf> {
        Entity::new(&self.root)
            .child(format!("users/{name}"))
            .into()
    }

    pub async fn create_user(
        &self,
        cx: &UserContext,
        meta: Meta,
        rec: &UserRecord,
    ) -> Result<User<'_>, CreateError<anyhow::Error>> {
        let user = self.user(cx);
        user.create_dir("").await?;
        try_join!(user.create_json(meta, rec), user.create_dir("repos"),)?;
        Ok(user)
    }

    pub fn repository<'a>(
        &'a self,
        RepositoryContext { owner, name }: &'a RepositoryContext,
    ) -> Repository<'_> {
        self.user(owner).repository(name)
    }

    pub fn tag<'a>(&'a self, TagContext { repository, name }: &'a TagContext) -> Tag<'_> {
        self.repository(repository).tag(name)
    }

    pub fn tree<'a>(&'a self, TreeContext { tag, path }: &'a TreeContext) -> Node<'_> {
        self.tag(tag).node(path)
    }
}

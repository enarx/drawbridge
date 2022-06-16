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

use drawbridge_type::{RepositoryContext, TagContext, TreeContext, UserContext};

use cap_async_std::fs_utf8::Dir;

#[derive(Debug)]
pub struct Store {
    root: Dir,
}

impl Store {
    pub fn user(&self, UserContext { name }: &UserContext) -> User<'_> {
        User::new(Entity::new(&self.root), name)
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
        self.tag(tag).path(path)
    }
}

impl From<Dir> for Store {
    #[inline]
    fn from(root: Dir) -> Self {
        Self { root }
    }
}

impl From<async_std::fs::File> for Store {
    #[inline]
    fn from(root: async_std::fs::File) -> Self {
        Self::from(Dir::from_std_file(root))
    }
}

impl From<std::fs::File> for Store {
    #[inline]
    fn from(root: std::fs::File) -> Self {
        Self::from(async_std::fs::File::from(root))
    }
}

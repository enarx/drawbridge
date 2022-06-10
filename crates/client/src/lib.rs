// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]

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

pub use drawbridge_type as types;

pub use anyhow::{Context, Result};
pub use mime;
pub use url::Url;

use drawbridge_type::{RepositoryContext, TagContext, TreeContext, UserContext};

#[derive(Debug)]
pub struct Client {
    inner: ureq::Agent,
    root: Url,
}

impl Client {
    pub fn builder(url: Url) -> ClientBuilder {
        ClientBuilder::new(url)
    }

    fn url(&self, path: &str) -> Result<Url> {
        self.root.join(path).context("failed to construct URL")
    }

    pub fn user(&self, UserContext { name }: &UserContext) -> User<'_> {
        User::new(Entity::new(self), name)
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

pub struct ClientBuilder {
    inner: ureq::AgentBuilder,
    url: Url,
}

impl ClientBuilder {
    pub fn new(url: Url) -> Self {
        Self {
            inner: ureq::AgentBuilder::new(),
            url,
        }
    }

    // TODO: Add configuration

    pub fn build(self) -> Client {
        Client {
            inner: self.inner.build(),
            root: self.url,
        }
    }
}

// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]

mod repo;
mod tag;
mod tree;

pub use repo::*;
pub use tag::*;
pub use tree::*;

pub use drawbridge_type as types;

pub use anyhow::Result;
pub use mime;
pub use url::Url;

use drawbridge_type::RepositoryName;

#[derive(Debug)]
pub struct Client {
    inner: ureq::Agent,
    url: Url,
}

impl Client {
    pub fn builder(url: impl Into<Url>) -> ClientBuilder {
        ClientBuilder::new(url)
    }

    pub fn repository<'a>(&'a self, name: &'a RepositoryName) -> Repository<'_> {
        Repository { client: self, name }
    }
}

pub struct ClientBuilder {
    inner: ureq::AgentBuilder,
    url: Url,
}

impl ClientBuilder {
    pub fn new(url: impl Into<Url>) -> Self {
        Self {
            inner: ureq::AgentBuilder::new(),
            url: url.into(),
        }
    }

    // TODO: Add configuration

    pub fn build(self) -> Client {
        Client {
            inner: self.inner.build(),
            url: self.url,
        }
    }
}

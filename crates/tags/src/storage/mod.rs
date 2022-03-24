// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

mod memory;

pub use memory::Memory;

use crate::hash::Hash;
use crate::tag::Tag;

use drawbridge_http::http::StatusCode;
use drawbridge_http::{async_trait, IntoResponse};

#[async_trait]
pub trait Storage: Send + Sync {
    type Error: IntoResponse + From<StatusCode>;

    async fn tags(&self) -> Result<Vec<Tag>, Self::Error>;

    async fn del(&self, tag: Tag) -> Result<(), Self::Error>;
    async fn get(&self, tag: Tag) -> Result<Hash, Self::Error>;
    async fn put(&self, tag: Tag, hash: Hash) -> Result<(), Self::Error>;
}

// TODO: Add tests

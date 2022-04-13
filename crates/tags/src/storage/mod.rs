// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

mod memory;

pub use memory::Memory;

use crate::Tag;

use axum::async_trait;

#[async_trait]
pub trait Storage: Send + Sync {
    type Error;

    async fn names(&self) -> Result<Vec<String>, Self::Error>;

    async fn del(&self, name: String) -> Result<(), Self::Error>;
    async fn get(&self, name: String) -> Result<Tag, Self::Error>;
    async fn put(&self, name: String, tag: Tag) -> Result<(), Self::Error>;
}

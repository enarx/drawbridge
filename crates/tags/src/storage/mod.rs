// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

mod memory;

pub use memory::Memory;

use crate::Tag;

use drawbridge_http::async_trait;
use drawbridge_http::http::Result;

#[async_trait]
pub trait Storage: Send + Sync {
    async fn names(&self) -> Result<Vec<String>>;

    async fn del(&self, name: String) -> Result<()>;
    async fn get(&self, name: String) -> Result<Tag>;
    async fn put(&self, name: String, tag: Tag) -> Result<()>;
}

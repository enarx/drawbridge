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

#[cfg(test)]
mod test {
    use super::{Hash, Memory, Storage, Tag};

    use std::ops::Deref;
    use std::str::FromStr;

    const HASH: &str = "sha256:LCa0a2j_xo_5m0U8HTBBNBNCLXBkg7-g-YpeiGJm564";

    #[async_std::test]
    async fn basic() {
        let m = Memory::default();
        assert_eq!(0, m.tags().await.unwrap().len());

        let tag = Tag::from_str("1.2.3").unwrap();
        let hash = Hash::from_str(HASH).unwrap();

        m.put(tag.clone(), hash).await.unwrap();
        let hash_retrieved = m.get(tag.clone()).await.unwrap();

        assert_eq!(HASH, hash_retrieved.deref().to_string());
        assert_eq!(1, m.tags().await.unwrap().len());
    }
}

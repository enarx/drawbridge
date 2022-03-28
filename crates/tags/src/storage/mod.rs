// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

mod memory;

pub use memory::Memory;

use crate::entry::Entry;
use crate::tag::Tag;

use drawbridge_http::http::StatusCode;
use drawbridge_http::{async_trait, IntoResponse};

#[async_trait]
pub trait Storage: Send + Sync {
    type Error: IntoResponse + From<StatusCode>;

    async fn tags(&self) -> Result<Vec<Tag>, Self::Error>;

    async fn del(&self, tag: Tag) -> Result<(), Self::Error>;
    async fn get(&self, tag: Tag) -> Result<Entry, Self::Error>;
    async fn put(&self, tag: Tag, entry: Entry) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod test {
    use super::{Entry, Hash, Memory, Storage, Tag};

    use std::str::FromStr;

    #[async_std::test]
    async fn basic() {
        let m = Memory::default();
        assert!(m.tags().await.unwrap().is_empty());

        let tag = Tag::from_str("1.2.3").unwrap();
        let entry = Entry::from_str("123").unwrap();

        m.put(tag.clone(), entry).await.unwrap();
        let entry_retrieved = m.get(tag.clone()).await.unwrap();

        assert_eq!("123", entry_retrieved.to_string());
        assert_eq!(vec![tag], m.tags().await.unwrap());
    }
}

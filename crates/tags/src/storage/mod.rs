// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

mod memory;

pub use memory::Memory;

use drawbridge_http::http::StatusCode;
use drawbridge_http::{async_trait, IntoResponse};

#[async_trait]
pub trait Storage: Send + Sync {
    type Error: IntoResponse + From<StatusCode>;

    async fn tags(&self) -> Result<Vec<String>, Self::Error>;

    async fn del(&self, tag: String) -> Result<(), Self::Error>;
    async fn get(&self, tag: String) -> Result<Vec<u8>, Self::Error>;
    async fn put(&self, tag: String, entry: Vec<u8>) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod test {
    use super::{Memory, Storage};

    #[async_std::test]
    async fn basic() {
        let m = Memory::default();
        assert!(m.tags().await.unwrap().is_empty());

        let tag = String::from("test_testing");
        let entry = tag.into_bytes();

        m.put(tag.clone(), entry.clone()).await.unwrap();
        let entry_retrieved = m.get(tag.clone()).await.unwrap();

        assert_eq!(tag.into_bytes(), entry_retrieved.to_string());
        assert_eq!(vec![tag], m.tags().await.unwrap());
    }
}

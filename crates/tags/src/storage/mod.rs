// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

mod memory;

pub use memory::Memory;

use crate::tag::{Name, Value};

use drawbridge_http::async_trait;
use drawbridge_http::http::Result;

#[async_trait]
pub trait Storage: Send + Sync {
    async fn tags(&self) -> Result<Vec<Name>>;

    async fn del(&self, tag: Name) -> Result<()>;
    async fn get(&self, tag: Name) -> Result<Value>;
    async fn put(&self, tag: Name, data: Value) -> Result<()>;
}

#[cfg(test)]
mod test {
    use super::{Memory, Storage};
    use crate::tag::{Kind, Name, Value};

    use std::ops::Deref;
    use std::str::FromStr;

    use drawbridge_type::Entry;

    #[async_std::test]
    async fn basic() {
        const ALGORITHM: &str = "sha-256";
        const HASH: &str = "4REjxQ4yrqUVicfSKYNO/cF9zNj5ANbzgDZt3/h3Qxo=";
        const NAME: &str = "example";

        let m = Memory::default();
        assert!(m.tags().await.unwrap().is_empty());

        let tag = Name::from_str(NAME).unwrap();
        let value = Value {
            body: vec![],
            kind: Kind::Unsigned(Entry {
                digest: format!("{}=:{}:", ALGORITHM, HASH).parse().unwrap(),
            }),
            name: tag.clone(),
        };

        m.put(tag.clone(), value).await.unwrap();
        let value_retrieved = m.get(tag.clone()).await.unwrap();

        assert_eq!(&NAME.to_string(), value_retrieved.name.deref());
        assert_eq!(vec![tag], m.tags().await.unwrap());
    }
}

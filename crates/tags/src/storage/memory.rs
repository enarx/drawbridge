// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::Storage;
use crate::hash::Hash;
use crate::tag::Tag;

use std::{collections::HashMap, sync::Arc};

use drawbridge_http::async_trait;
use drawbridge_http::http::StatusCode;

use async_std::sync::RwLock;

/// A memory-backed storage driver.
///
/// This is mostly for testing.
#[derive(Clone)]
pub struct Memory(Arc<RwLock<HashMap<Tag, Hash>>>);

impl Default for Memory {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }
}

#[async_trait]
impl Storage for Memory {
    type Error = StatusCode;

    async fn tags(&self) -> Result<Vec<Tag>, Self::Error> {
        let our_map = self.0.read().await;
        Ok(our_map.keys().cloned().collect())
    }

    async fn del(&self, tag: Tag) -> Result<(), Self::Error> {
        let mut our_map = self.0.write().await;
        our_map.remove(&tag).ok_or(StatusCode::NotFound)?;
        Ok(())
    }

    async fn get(&self, tag: Tag) -> Result<Hash, Self::Error> {
        let our_map = self.0.read().await;
        let x = our_map.get(&tag).ok_or(StatusCode::NotFound)?;
        Ok(x.clone())
    }

    async fn put(&self, tag: Tag, hash: Hash) -> Result<(), Self::Error> {
        let mut our_map = self.0.write().await;
        our_map.insert(tag, hash);
        Ok(())
    }
}

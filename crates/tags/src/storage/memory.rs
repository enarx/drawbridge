// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::Storage;

use std::{collections::HashMap, sync::Arc};

use drawbridge_http::async_trait;
use drawbridge_http::http::StatusCode;

use async_std::sync::RwLock;

/// A memory-backed storage driver.
///
/// This is mostly for testing.
#[derive(Clone)]
pub struct Memory(Arc<RwLock<HashMap<String, Vec<u8>>>>);

impl Default for Memory {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }
}

#[async_trait]
impl Storage for Memory {
    type Error = StatusCode;

    async fn tags(&self) -> Result<Vec<String>, Self::Error> {
        let lock = self.0.read().await;
        Ok(lock.keys().cloned().collect())
    }

    async fn del(&self, tag: String) -> Result<(), Self::Error> {
        let mut lock = self.0.write().await;
        lock.remove(&tag).ok_or(StatusCode::NotFound)?;
        Ok(())
    }

    async fn get(&self, tag: String) -> Result<Vec<u8>, Self::Error> {
        let lock = self.0.read().await;
        let x = lock.get(&tag).ok_or(StatusCode::NotFound)?;
        Ok(x.clone())
    }

    async fn put(&self, tag: String, entry: Vec<u8>) -> Result<(), Self::Error> {
        let mut lock = self.0.write().await;
        lock.insert(tag, entry);
        Ok(())
    }
}

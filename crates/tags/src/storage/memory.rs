// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use axum::{async_trait, http::StatusCode};
use tokio::sync::RwLock;

use super::Storage;
use crate::Tag;

use std::collections::HashMap;
use std::sync::Arc;

/// A memory-backed storage driver.
///
/// This is mostly for testing.
#[derive(Clone)]
pub struct Memory(Arc<RwLock<HashMap<String, Tag>>>);

impl Default for Memory {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }
}

#[async_trait]
impl Storage for Memory {
    type Error = (StatusCode, &'static str);

    async fn names(&self) -> Result<Vec<String>, Self::Error> {
        let lock = self.0.read().await;
        Ok(lock.keys().cloned().collect())
    }

    async fn del(&self, name: String) -> Result<(), Self::Error> {
        let mut lock = self.0.write().await;
        lock.remove(&name).ok_or((StatusCode::NOT_FOUND, ""))?;
        Ok(())
    }

    async fn get(&self, name: String) -> Result<Tag, Self::Error> {
        let lock = self.0.read().await;
        let x = lock.get(&name).ok_or((StatusCode::NOT_FOUND, ""))?;
        Ok(x.clone())
    }

    async fn put(&self, name: String, tag: Tag) -> Result<(), Self::Error> {
        let mut lock = self.0.write().await;
        lock.insert(name, tag);
        Ok(())
    }
}

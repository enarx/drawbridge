// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::Storage;
use crate::tag::{Name, Value};

use std::{collections::HashMap, sync::Arc};

use drawbridge_http::async_trait;
use drawbridge_http::http::{Error, Result, StatusCode};

use async_std::sync::RwLock;

/// A memory-backed storage driver.
///
/// This is mostly for testing.
#[derive(Clone)]
pub struct Memory(Arc<RwLock<HashMap<Name, Value>>>);

impl Default for Memory {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }
}

#[async_trait]
impl Storage for Memory {
    async fn tags(&self) -> Result<Vec<Name>> {
        let lock = self.0.read().await;
        Ok(lock.keys().cloned().collect())
    }

    async fn del(&self, tag: Name) -> Result<()> {
        let mut lock = self.0.write().await;
        lock.remove(&tag)
            .ok_or_else(|| Error::from_str(StatusCode::NotFound, ""))?;
        Ok(())
    }

    async fn get(&self, tag: Name) -> Result<Value> {
        let lock = self.0.read().await;
        let x = lock
            .get(&tag)
            .ok_or_else(|| Error::from_str(StatusCode::NotFound, ""))?;
        Ok(x.clone())
    }

    async fn put(&self, tag: Name, data: Value) -> Result<()> {
        let mut lock = self.0.write().await;
        lock.insert(tag, data);
        Ok(())
    }
}

// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

mod memory;

use axum::http::StatusCode;
pub use memory::Memory;

use crate::node::Node;
use crate::path::Path;

use drawbridge_hash::Hash;
use drawbridge_type::Meta;

use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};

use axum::async_trait;
use axum::body::Body;
use futures::AsyncRead;

use serde::Deserialize;

/// A directory
#[derive(Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Directory(BTreeMap<String, Entry>);

impl Directory {
    pub const TYPE: &'static str = "application/vnd.drawbridge.directory.v1+json";
}

impl Deref for Directory {
    type Target = BTreeMap<String, Entry>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Directory {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A directory entry
///
/// Note that this type is designed to be extensible. Therefore, the fields
/// here represent the minimum required fields. Other fields may be present.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Entry {
    /// The hash of this entry
    pub hash: Hash,
}

impl Entry {
    #[allow(dead_code)]
    pub const TYPE: &'static str = "application/vnd.drawbridge.entry.v1+json";
}

#[async_trait]
pub trait Storage: Send + Sync {
    async fn roots(&self) -> Result<Vec<Node>, (StatusCode, &'static str)>;
    async fn wants(&self, path: Path) -> Result<Vec<Node>, (StatusCode, &'static str)>;

    async fn del(&self, path: Path) -> Result<(), (StatusCode, &'static str)>;
    async fn get(&self, path: Path) -> Result<(Meta, Body), (StatusCode, &'static str)>;
    async fn put<T>(
        &self,
        path: Path,
        meta: Meta,
        body: T,
    ) -> Result<(), (StatusCode, &'static str)>
    where
        T: Send + AsyncRead + Unpin;
}

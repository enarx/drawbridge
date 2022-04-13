// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::{Directory, Storage};
use crate::node::Node;
use crate::path::Path;

use drawbridge_type::Meta;

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use axum::async_trait;
use axum::body::Body;
use axum::http::StatusCode;
use futures::{AsyncRead, AsyncReadExt};
use tokio::sync::RwLock;

#[derive(PartialEq, Eq)]
enum Segment {
    Directory(HashMap<Node, Segment>, Meta, Vec<u8>),
    File(Meta, Vec<u8>),
    None,
}

impl Segment {
    fn unwind(&self, path: &[Node]) -> Result<&Segment, (StatusCode, &'static str)> {
        let mut here = self;

        for node in path.iter() {
            here = match here {
                Segment::Directory(map, ..) => map.get(node).ok_or((StatusCode::NOT_FOUND, ""))?,
                Segment::File(..) => return Err((StatusCode::BAD_REQUEST, "")),
                Segment::None => return Err((StatusCode::NOT_FOUND, "")),
            };
        }

        Ok(here)
    }

    fn unwind_mut(&mut self, path: &[Node]) -> Result<&mut Segment, (StatusCode, &'static str)> {
        let mut here = self;

        for node in path.iter() {
            here = match here {
                Segment::Directory(map, ..) => map
                    .get_mut(node)
                    .ok_or_else(|| (StatusCode::NOT_FOUND, ""))?,
                Segment::File(..) => return Err((StatusCode::BAD_REQUEST, "")),
                Segment::None => return Err((StatusCode::NOT_FOUND, "")),
            };
        }

        Ok(here)
    }

    fn map(&self, path: &[Node]) -> Result<&HashMap<Node, Segment>, (StatusCode, &'static str)> {
        match self.unwind(path)? {
            Segment::Directory(ref map, ..) => Ok(map),
            Segment::File(..) => Err((StatusCode::BAD_REQUEST, "")),
            Segment::None => Err((StatusCode::NOT_FOUND, "")),
        }
    }

    fn map_mut(
        &mut self,
        path: &[Node],
    ) -> Result<&mut HashMap<Node, Segment>, (StatusCode, &'static str)> {
        match self.unwind_mut(path)? {
            Segment::Directory(ref mut map, ..) => Ok(map),
            Segment::File(..) => Err((StatusCode::BAD_REQUEST, "")),
            Segment::None => Err((StatusCode::NOT_FOUND, "")),
        }
    }
}

/// A memory-backed storage driver.
///
/// This is mostly for testing. No attempt at deduplication is performed.
#[derive(Clone)]
pub struct Memory(Arc<RwLock<Segment>>);

impl Default for Memory {
    fn default() -> Self {
        const HASH: &str = "sha256:LCa0a2j_xo_5m0U8HTBBNBNCLXBkg7-g-YpeiGJm564";

        // Ignore the actual values here. Only the HashMap is used.
        Self(Arc::new(RwLock::new(Segment::Directory(
            HashMap::new(),
            Meta {
                hash: HASH.parse().unwrap(),
                mime: Directory::TYPE.parse().unwrap(),
                size: 0,
            },
            Vec::new(),
        ))))
    }
}

#[async_trait]
impl Storage for Memory {
    async fn roots(&self) -> Result<Vec<Node>, (StatusCode, &'static str)> {
        let lock = self.0.read().await;
        match &*lock {
            Segment::Directory(map, ..) => Ok(map.keys().cloned().collect()),
            _ => Err((StatusCode::INTERNAL_SERVER_ERROR, "")),
        }
    }

    async fn wants(&self, path: Path) -> Result<Vec<Node>, (StatusCode, &'static str)> {
        let lock = self.0.read().await;
        let iter = lock.map(&path[..])?.iter();
        let none = iter.filter(|(_, v)| v == &&Segment::None).map(|(k, _)| k);
        Ok(none.cloned().collect())
    }

    async fn del(&self, path: Path) -> Result<(), (StatusCode, &'static str)> {
        if path.len() != 1 {
            return Err((StatusCode::BAD_REQUEST, ""));
        }

        let mut lock = self.0.write().await;
        match lock.map_mut(&[])?.remove(&path[0]) {
            Some(..) => Ok(()),
            None => Err((StatusCode::NOT_FOUND, "")),
        }
    }

    async fn get(&self, path: Path) -> Result<(Meta, Body), (StatusCode, &'static str)> {
        let lock = self.0.read().await;
        let (meta, data) = match lock.unwind(&path)? {
            Segment::Directory(_, meta, data) => (meta, data),
            Segment::File(meta, data) => (meta, data),
            Segment::None => return Err((StatusCode::NOT_FOUND, "")),
        };

        Ok((meta.clone(), Body::from(data.clone())))
    }

    async fn put<T>(
        &self,
        path: Path,
        meta: Meta,
        mut body: T,
    ) -> Result<(), (StatusCode, &'static str)>
    where
        T: Send + AsyncRead + Unpin,
    {
        let mut lock = self.0.write().await;
        let map = lock.map_mut(&path[..path.len() - 1])?;

        // If we have a new tree root, insert `Segment::None`.
        let last = path.last().unwrap();
        if path.len() == 1 {
            map.insert(last.clone(), Segment::None);
        }

        // Reject uploads if the node is unknown or already exists.
        match map.get(last) {
            Some(Segment::None) => (),
            None => return Err((StatusCode::NOT_FOUND, "")),
            _ => return Err((StatusCode::BAD_REQUEST, "")),
        }

        // Read the body into memory.
        let mut data = Vec::new();
        Pin::new(&mut body)
            .read_to_end(&mut data)
            .await
            .map_err(|_| (StatusCode::BAD_REQUEST, ""))?;

        // Validate the size against the Content-Length.
        if data.len() as u64 != meta.size {
            return Err((StatusCode::BAD_REQUEST, ""));
        }

        // If this is a file, we are done.
        if meta.mime.essence_str() != Directory::TYPE {
            map.insert(last.clone(), Segment::File(meta, data));
            return Ok(());
        }

        // Create a directory entry.
        let dir: Directory =
            serde_json::from_slice(&data).map_err(|_| (StatusCode::BAD_REQUEST, ""))?;

        // Populate children nodes with unknown states.
        let mut new = HashMap::new();
        for entry in dir.values() {
            new.insert(entry.hash.clone().into(), Segment::None);
        }

        // Insert the directory.
        map.insert(last.clone(), Segment::Directory(new, meta, data));
        Ok(())
    }
}

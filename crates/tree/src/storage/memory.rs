// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::{Directory, Storage};
use crate::{meta::Meta, node::Node, path::Path};

use std::{collections::HashMap, pin::Pin, sync::Arc};

use drawbridge_http::async_trait;
use drawbridge_http::http::{Body, Error, Result, StatusCode};

use async_std::io::{Read, ReadExt};
use async_std::sync::RwLock;

#[derive(PartialEq, Eq)]
enum Segment {
    Directory(HashMap<Node, Segment>, Meta, Vec<u8>),
    File(Meta, Vec<u8>),
    None,
}

impl Segment {
    fn unwind(&self, path: &[Node]) -> Result<&Segment> {
        let mut here = self;

        for node in path.iter() {
            here = match here {
                Segment::Directory(map, ..) => map
                    .get(node)
                    .ok_or_else(|| Error::from_str(StatusCode::NotFound, ""))?,
                Segment::File(..) => return Err(Error::from_str(StatusCode::BadRequest, "")),
                Segment::None => return Err(Error::from_str(StatusCode::NotFound, "")),
            };
        }

        Ok(here)
    }

    fn unwind_mut(&mut self, path: &[Node]) -> Result<&mut Segment> {
        let mut here = self;

        for node in path.iter() {
            here = match here {
                Segment::Directory(map, ..) => map
                    .get_mut(node)
                    .ok_or_else(|| Error::from_str(StatusCode::NotFound, ""))?,
                Segment::File(..) => return Err(Error::from_str(StatusCode::BadRequest, "")),
                Segment::None => return Err(Error::from_str(StatusCode::NotFound, "")),
            };
        }

        Ok(here)
    }

    fn map(&self, path: &[Node]) -> Result<&HashMap<Node, Segment>> {
        match self.unwind(path)? {
            Segment::Directory(map, ..) => Ok(map),
            Segment::File(..) => Err(Error::from_str(StatusCode::BadRequest, "")),
            Segment::None => Err(Error::from_str(StatusCode::NotFound, "")),
        }
    }

    fn map_mut(&mut self, path: &[Node]) -> Result<&mut HashMap<Node, Segment>> {
        match self.unwind_mut(path)? {
            Segment::Directory(map, ..) => Ok(map),
            Segment::File(..) => Err(Error::from_str(StatusCode::BadRequest, "")),
            Segment::None => Err(Error::from_str(StatusCode::NotFound, "")),
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
    async fn roots(&self) -> Result<Vec<Node>> {
        let lock = self.0.read().await;
        match &*lock {
            Segment::Directory(map, ..) => Ok(map.keys().cloned().collect()),
            _ => Err(Error::from_str(StatusCode::InternalServerError, "")),
        }
    }

    async fn wants(&self, path: Path) -> Result<Vec<Node>> {
        let lock = self.0.read().await;
        let iter = lock.map(&path[..])?.iter();
        let none = iter.filter(|(_, v)| v == &&Segment::None).map(|(k, _)| k);
        Ok(none.cloned().collect())
    }

    async fn del(&self, path: Path) -> Result<()> {
        if path.len() != 1 {
            return Err(Error::from_str(StatusCode::BadRequest, ""));
        }

        let mut lock = self.0.write().await;
        match lock.map_mut(&[])?.remove(&path[0]) {
            Some(..) => Ok(()),
            None => Err(Error::from_str(StatusCode::NotFound, "")),
        }
    }

    async fn get(&self, path: Path) -> Result<(Meta, Body)> {
        let lock = self.0.read().await;
        let (meta, data) = match lock.unwind(&path)? {
            Segment::Directory(_, meta, data) => (meta, data),
            Segment::File(meta, data) => (meta, data),
            Segment::None => return Err(Error::from_str(StatusCode::NotFound, "")),
        };

        Ok((meta.clone(), Body::from_bytes(data.clone())))
    }

    async fn put<T>(&self, path: Path, meta: Meta, mut body: T) -> Result<()>
    where
        T: Send + Read + Unpin,
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
            None => return Err(Error::from_str(StatusCode::NotFound, "")),
            _ => return Err(Error::from_str(StatusCode::BadRequest, "")),
        }

        // Read the body into memory.
        let mut data = Vec::new();
        Pin::new(&mut body)
            .read_to_end(&mut data)
            .await
            .map_err(|_| Error::from_str(StatusCode::BadRequest, ""))?;

        // Validate the size against the Content-Length.
        if data.len() as u64 != meta.size {
            return Err(Error::from_str(StatusCode::BadRequest, ""));
        }

        // If this is a file, we are done.
        if meta.mime.essence() != Directory::TYPE {
            map.insert(last.clone(), Segment::File(meta, data));
            return Ok(());
        }

        // Create a directory entry.
        let dir: Directory = serde_json::from_slice(&data)
            .map_err(|_| Error::from_str(StatusCode::BadRequest, ""))?;

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

// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{Create, CreateError, CreateItem, Get, GetError, Keys};

use std::collections::hash_map::{Entry, VacantEntry};
use std::collections::HashMap;
use std::convert::Infallible;
use std::hash::Hash;
use std::pin::Pin;
use std::task::{Context, Poll};

use drawbridge_type::digest::{Algorithms, ContentDigest};
use drawbridge_type::Meta;

use async_trait::async_trait;
use futures::io::{self, copy};
use futures::stream::{iter, Iter};
use futures::AsyncWrite;
use mime::Mime;

#[derive(Clone)]
pub struct Memory<K>(HashMap<K, (Mime, ContentDigest, Vec<u8>)>);

impl<K> Default for Memory<K> {
    fn default() -> Self {
        Self(Default::default())
    }
}

pub struct MemoryCreateItem<'a, K> {
    mime: Mime,
    buf: Vec<u8>,
    entry: VacantEntry<'a, K, (Mime, ContentDigest, Vec<u8>)>,
}

impl<K> AsyncWrite for MemoryCreateItem<'_, K>
where
    K: Unpin,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.buf).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.buf).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.buf).poll_close(cx)
    }
}

#[async_trait]
impl<K> CreateItem for MemoryCreateItem<'_, K>
where
    K: Send + Unpin,
{
    async fn finish(self) -> (u64, ContentDigest) {
        let size = self.buf.len() as _;

        // TODO: Compute hash while writing
        let mut buf: Vec<u8> = Vec::with_capacity(self.buf.len());
        let mut hasher = Algorithms::default().writer(&mut buf);
        copy(&mut self.buf.as_slice(), &mut hasher).await.unwrap();
        let hash = hasher.digests();

        self.entry.insert((self.mime, hash.clone(), buf));
        (size, hash)
    }
}

#[async_trait]
impl<K> Create<K> for Memory<K>
where
    K: Sync + Send + Unpin + Eq + Hash,
{
    type Item<'a> = MemoryCreateItem<'a, K> where K: 'a;
    type Error = Infallible;

    async fn create(
        &mut self,
        key: K,
        mime: Mime,
    ) -> Result<Self::Item<'_>, CreateError<Self::Error>> {
        match self.0.entry(key) {
            Entry::Vacant(entry) => Ok(MemoryCreateItem {
                mime,
                buf: vec![],
                entry,
            }),
            Entry::Occupied(_) => Err(CreateError::Occupied),
        }
    }
}

#[async_trait]
impl<K> Get<K> for Memory<K>
where
    K: Sync + Send + Eq + Hash,
{
    type Item<'a> = &'a [u8] where K:'a;
    type Error = Infallible;

    async fn get(&self, key: K) -> Result<(Meta, Self::Item<'_>), GetError<Self::Error>> {
        self.0
            .get(&key)
            .ok_or(GetError::NotFound)
            .map(|(mime, hash, v)| {
                (
                    Meta {
                        mime: mime.clone(),
                        hash: hash.clone(),
                        size: v.len() as _,
                    },
                    v.as_slice(),
                )
            })
    }
}

#[async_trait]
impl<K> Keys<K> for Memory<K>
where
    K: Sync + Send + Clone,
{
    type Stream = Iter<std::vec::IntoIter<Result<K, Infallible>>>;
    type StreamError = Infallible;

    async fn keys(&self) -> Self::Stream {
        iter(self.0.keys().cloned().map(Ok).collect::<Vec<_>>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use futures::{AsyncReadExt, AsyncWriteExt, StreamExt};

    #[tokio::test]
    async fn test() {
        let key = "test";
        let mime = "text/plain".parse::<Mime>().unwrap();

        let mut mem = Memory::default();

        assert_eq!(mem.get(key).await, Err(GetError::NotFound));

        assert!(mem.keys().await.collect::<Vec<_>>().await.is_empty());

        let hash = {
            let mut hasher = Algorithms::default().writer(io::sink());
            assert!(matches!(hasher.write_all(&[42]).await, Ok(())));
            hasher.digests()
        };

        {
            let w = mem.create(key, mime.clone()).await;
            assert!(matches!(w, Ok(_)));
            let mut w = w.unwrap();
            assert!(matches!(w.write_all(&[42]).await, Ok(())));
            assert_eq!(w.finish().await, (1, hash.clone()))
        }

        {
            let mr = mem.get(key).await;
            assert!(matches!(mr, Ok(_)));
            let mut v = vec![];
            let (meta, mut r) = mr.unwrap();
            assert_eq!(
                meta,
                Meta {
                    mime: mime.clone(),
                    size: 1,
                    hash,
                }
            );
            assert!(matches!(r.read_to_end(&mut v).await, Ok(1)))
        }

        assert!(matches!(
            mem.create(key, mime).await,
            Err(CreateError::Occupied)
        ));

        assert_eq!(mem.keys().await.collect::<Vec<_>>().await, vec![Ok(key)]);
    }
}

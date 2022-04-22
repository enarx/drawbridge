// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]
#![feature(generic_associated_types)]

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
use futures::{AsyncRead, AsyncWrite, AsyncWriteExt, TryFutureExt, TryStream};
use mime::Mime;

#[derive(Clone, Debug, PartialEq)]
pub enum CreateError<E> {
    Occupied,
    Internal(E),
}

#[derive(Debug)]
pub enum CreateCopyError<E> {
    IO(io::Error),
    Create(CreateError<E>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum GetError<E> {
    NotFound,
    Internal(E),
}

#[async_trait]
pub trait CreateItem: AsyncWrite {
    async fn finish(self) -> (u64, ContentDigest);
}

#[async_trait]
pub trait Create<K>
where
    K: Send,
{
    type Item<'a>: Sync + Send + Unpin + CreateItem
    where
        Self: 'a;

    type Error: Sync + Send + std::error::Error;

    async fn create(
        &mut self,
        key: K,
        mime: Mime,
    ) -> Result<Self::Item<'_>, CreateError<Self::Error>>
    where
        K: 'async_trait;

    async fn create_copy<R>(
        &mut self,
        key: K,
        mime: Mime,
        src: R,
    ) -> Result<(u64, ContentDigest), CreateCopyError<Self::Error>>
    where
        Self: 'static,
        K: 'async_trait,
        R: Send + AsyncRead + 'async_trait,
    {
        let mut w = self
            .create(key, mime)
            .await
            .map_err(CreateCopyError::Create)?;
        copy(src, &mut w).await.map_err(CreateCopyError::IO)?;
        Ok(w.finish().await)
    }
}

#[async_trait]
pub trait Get<K>
where
    K: Send,
{
    type Item<'a>: Sync + Send + Unpin + AsyncRead
    where
        Self: 'a;
    type Error: Sync + Send + std::error::Error;

    async fn get(&self, k: K) -> Result<(Meta, Self::Item<'_>), GetError<Self::Error>>
    where
        K: 'async_trait;

    async fn get_meta(&self, k: K) -> Result<Meta, GetError<Self::Error>>
    where
        K: 'async_trait,
    {
        self.get(k).await.map(|(m, _)| m)
    }

    async fn contains(&self, k: K) -> Result<bool, Self::Error>
    where
        K: 'async_trait,
    {
        self.get(k)
            .map_ok_or_else(
                |e| match e {
                    GetError::Internal(e) => Err(e),
                    GetError::NotFound => Ok(false),
                },
                |_| Ok(true),
            )
            .await
    }
}

#[async_trait]
pub trait Keys<K> {
    type Stream: Send + TryStream<Ok = K, Error = Self::StreamError>;
    type StreamError: Sync + Send + std::error::Error;

    type Error: Sync + Send + std::error::Error;

    async fn keys(&self) -> Result<Self::Stream, Self::Error>;
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

#[derive(Clone)]
pub struct Memory<K>(HashMap<K, (Mime, ContentDigest, Vec<u8>)>);

impl<K> Default for Memory<K> {
    fn default() -> Self {
        Self(Default::default())
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
        if let Entry::Vacant(entry) = self.0.entry(key) {
            let mut buf = vec![];
            Ok(MemoryCreateItem { mime, buf, entry })
        } else {
            Err(CreateError::Occupied)
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
    type Stream = Iter<std::vec::IntoIter<Result<K, Self::StreamError>>>;
    type StreamError = Infallible;
    type Error = Infallible;

    async fn keys(&self) -> Result<Self::Stream, Self::Error> {
        Ok(iter(self.0.keys().cloned().map(Ok).collect::<Vec<_>>()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use futures::{AsyncReadExt, AsyncWriteExt, StreamExt};

    #[tokio::test]
    async fn test_memory() {
        let key = "test";
        let mime = "text/plain".parse::<Mime>().unwrap();

        let mut mem = Memory::default();

        assert_eq!(mem.get(key).await, Err(GetError::NotFound));

        {
            let st = mem.keys().await;
            assert!(matches!(st, Ok(_)));
            assert!(st.unwrap().collect::<Vec<_>>().await.is_empty());
        }

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

        {
            let st = mem.keys().await;
            assert!(matches!(st, Ok(_)));
            assert_eq!(st.unwrap().collect::<Vec<_>>().await, vec![Ok(key)]);
        }
    }
}

// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::convert::Infallible;
use std::hash::Hash;

use drawbridge_type::Meta;

use async_trait::async_trait;
use futures::io::{self, copy};
use futures::stream::{iter, Iter};
use futures::{AsyncRead, AsyncWrite, TryStream};

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
pub trait Store<K>
where
    K: Send,
    for<'a> &'a mut Self::Write: AsyncWrite,
    for<'a> &'a Self::Read: AsyncRead,
{
    type Read: ?Sized + Sync;
    type Write: ?Sized + Sync + Send;
    type Error: Sync + Send + std::error::Error;

    async fn create(&mut self, k: K, m: Meta) -> Result<&mut Self::Write, CreateError<Self::Error>>
    where
        K: 'async_trait;

    async fn create_copy<R>(
        &mut self,
        k: K,
        m: Meta,
        src: R,
    ) -> Result<u64, CreateCopyError<Self::Error>>
    where
        K: 'async_trait,
        R: Send + AsyncRead + 'async_trait,
    {
        let mut bw = self.create(k, m).await.map_err(CreateCopyError::Create)?;
        copy(src, &mut bw).await.map_err(CreateCopyError::IO)
    }

    async fn get(&self, k: K) -> Result<(Meta, &Self::Read), GetError<Self::Error>>
    where
        K: 'async_trait;

    async fn get_meta(&self, k: K) -> Result<Meta, GetError<Self::Error>>
    where
        K: 'async_trait,
    {
        self.get(k).await.map(|(m, _)| m)
    }
}

#[async_trait]
pub trait Keys<K> {
    type Stream: Send + TryStream<Ok = K, Error = Self::StreamError>;
    type StreamError: Sync + Send + std::error::Error;

    type Error: Sync + Send + std::error::Error;

    async fn keys(&self) -> Result<Self::Stream, Self::Error>;
}

#[derive(Default, Clone)]
pub struct Memory<K>(HashMap<K, (Meta, Vec<u8>)>);

#[async_trait]
impl<K> Store<K> for Memory<K>
where
    K: Sync + Send + Eq + Hash,
{
    type Read = [u8];
    type Write = Vec<u8>;
    type Error = Infallible;

    async fn create(
        &mut self,
        k: K,
        m: Meta,
    ) -> Result<&mut Self::Write, CreateError<Self::Error>> {
        if let Entry::Vacant(e) = self.0.entry(k) {
            Ok(&mut e.insert((m, vec![])).1)
        } else {
            Err(CreateError::Occupied)
        }
    }

    async fn get(&self, k: K) -> Result<(Meta, &Self::Read), GetError<Self::Error>> {
        self.0
            .get(&k)
            .ok_or(GetError::NotFound)
            .map(|(m, v)| (m.clone(), v.as_slice()))
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
        let meta = Meta {
            hash: "sha-384=:mqVuAfXRKap7bdgcCY5uykM6+R9GqQ8K/uxy9rx7HNQlGYl1kPzQho1wx4JwY8w=:"
                .parse()
                .unwrap(),
            size: 42,
            mime: "text/plain".parse().unwrap(),
        };

        let mut mem = Memory::default();

        assert_eq!(mem.get(key).await, Err(GetError::NotFound));

        {
            let st = mem.keys().await;
            assert!(matches!(st, Ok(_)));
            assert!(st.unwrap().collect::<Vec<_>>().await.is_empty());
        }

        {
            let w = mem.create(key, meta.clone()).await;
            assert!(matches!(w, Ok(_)));
            assert!(matches!(w.unwrap().write_all(&[42]).await, Ok(())));
        }

        {
            let mr = mem.get(key).await;
            assert!(matches!(mr, Ok(_)));
            let mut v = vec![];
            let (m, mut r) = mr.unwrap();
            assert_eq!(m, meta);
            assert!(matches!(r.read_to_end(&mut v).await, Ok(1)))
        }

        assert!(matches!(
            mem.create(key, meta.clone()).await,
            Err(CreateError::Occupied)
        ));

        {
            let st = mem.keys().await;
            assert!(matches!(st, Ok(_)));
            assert_eq!(st.unwrap().collect::<Vec<_>>().await, vec![Ok(key)]);
        }
    }
}

// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

mod memory;

pub use memory::*;

use drawbridge_type::digest::ContentDigest;
use drawbridge_type::Meta;

use async_trait::async_trait;
use futures::io::{self, copy};
use futures::{AsyncRead, AsyncWrite, Stream, TryFutureExt};
use mime::Mime;

#[derive(Clone, Debug, PartialEq)]
pub enum CreateError<E> {
    Occupied,
    Internal(E),
}

#[derive(Debug)]
pub enum CreateFromReaderError<E> {
    IO(io::Error),
    Create(CreateError<E>),
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

    async fn create_from_reader<R>(
        &mut self,
        key: K,
        mime: Mime,
        src: R,
    ) -> Result<(u64, ContentDigest), CreateFromReaderError<Self::Error>>
    where
        Self: 'static,
        K: 'async_trait,
        R: Send + AsyncRead + 'async_trait,
    {
        let mut w = self
            .create(key, mime)
            .await
            .map_err(CreateFromReaderError::Create)?;
        copy(src, &mut w).await.map_err(CreateFromReaderError::IO)?;
        Ok(w.finish().await)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum GetError<E> {
    NotFound,
    Internal(E),
}

#[derive(Debug)]
pub enum GetToWriterError<E> {
    IO(io::Error),
    Get(GetError<E>),
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

    async fn get_to_writer<W>(
        &self,
        k: K,
        dst: &mut W,
    ) -> Result<Meta, GetToWriterError<Self::Error>>
    where
        Self: 'static,
        K: 'async_trait,
        W: Send + Unpin + AsyncWrite + 'async_trait,
    {
        let (meta, r) = self.get(k).await.map_err(GetToWriterError::Get)?;
        copy(r, dst).await.map_err(GetToWriterError::IO)?;
        Ok(meta)
    }

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
    type Stream: Send + Stream<Item = Result<K, Self::StreamError>>;
    type StreamError: Sync + Send + std::error::Error;

    async fn keys(&self) -> Self::Stream;
}

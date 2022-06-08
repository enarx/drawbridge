// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::io;
use std::os::unix::fs::DirBuilderExt;

use drawbridge_type::Meta;

use anyhow::Context;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use camino::{Utf8Path, Utf8PathBuf};
use cap_async_std::fs_utf8::{Dir, DirBuilder, ReadDir};
use futures::future::TryFutureExt;
use futures::io::copy;
use futures::try_join;
use futures::{AsyncRead, AsyncWrite};
use serde::Serialize;

const STORAGE_FAILURE_RESPONSE: (StatusCode, &str) =
    (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure");

#[derive(Debug)]
pub enum CreateError<E> {
    EmptyDigest,
    Occupied,
    LengthMismatch { expected: u64, got: u64 },
    DigestMismatch,
    Internal(E),
}

impl<E> IntoResponse for CreateError<E> {
    fn into_response(self) -> Response {
        match self {
            CreateError::EmptyDigest => (
                StatusCode::BAD_REQUEST,
                "At least one content digest value must be specified",
            )
                .into_response(),
            CreateError::Occupied => (StatusCode::CONFLICT, "Already exists").into_response(),
            CreateError::DigestMismatch => {
                (StatusCode::BAD_REQUEST, "Content digest mismatch").into_response()
            }
            CreateError::LengthMismatch { expected, got } => (
                StatusCode::BAD_REQUEST,
                format!(
                    "Content length mismatch, expected: {}, got {}",
                    expected, got
                ),
            )
                .into_response(),
            CreateError::Internal(_) => STORAGE_FAILURE_RESPONSE.into_response(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum GetError<E> {
    NotFound,
    Internal(E),
}

impl<E> IntoResponse for GetError<E> {
    fn into_response(self) -> Response {
        match self {
            GetError::NotFound => (StatusCode::NOT_FOUND, "Not found"),
            GetError::Internal(_) => STORAGE_FAILURE_RESPONSE,
        }
        .into_response()
    }
}

#[derive(Debug)]
pub enum GetToWriterError<E> {
    IO(io::Error),
    Get(GetError<E>),
}

impl<E> IntoResponse for GetToWriterError<E> {
    fn into_response(self) -> Response {
        match self {
            GetToWriterError::Get(GetError::NotFound) => {
                (StatusCode::NOT_FOUND, "Repository does not exist")
            }
            GetToWriterError::Get(GetError::Internal(_)) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
            }
            GetToWriterError::IO(_) => (StatusCode::INTERNAL_SERVER_ERROR, "I/O error"),
        }
        .into_response()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Entity<'a, P> {
    root: &'a Dir,
    prefix: P,
}

/// Sets default options on a [DirBuilder]
pub fn dir_builder_defaults(dir_builder: &mut DirBuilder) -> &mut DirBuilder {
    dir_builder.recursive(false).mode(0o700)
}

impl<'a, P: AsRef<Utf8Path>> Entity<'a, P> {
    pub fn new(root: &'a Dir, path: P) -> Self {
        Self { root, prefix: path }
    }

    /// Returns a child [Entity]
    pub fn child(&self, path: impl AsRef<Utf8Path>) -> Entity<'a, Utf8PathBuf> {
        Entity::new(self.root, self.path(path))
    }

    fn path(&self, path: impl AsRef<Utf8Path>) -> Utf8PathBuf {
        self.prefix.as_ref().join(path)
    }

    fn meta_path(&self) -> Utf8PathBuf {
        self.path("meta.json")
    }

    fn content_path(&self) -> Utf8PathBuf {
        self.path("content")
    }

    pub(super) async fn create_from_reader_with(
        &self,
        meta: Meta,
        rdr: impl Unpin + AsyncRead,
        dir_builder: &DirBuilder,
    ) -> Result<(), CreateError<anyhow::Error>> {
        if meta.hash.is_empty() {
            return Err(CreateError::EmptyDigest);
        }

        self.create_dir_with("", dir_builder).await?;

        let meta_json = serde_json::to_vec(&meta)
            .context("failed to encode metadata")
            .map_err(CreateError::Internal)?;
        let ((), mut cont) = try_join!(
            self.root
                .write(self.meta_path(), meta_json)
                .map_err(|e| match e.kind() {
                    io::ErrorKind::AlreadyExists => CreateError::Occupied,
                    _ => CreateError::Internal(
                        anyhow::Error::new(e).context("failed to write metadata"),
                    ),
                }),
            self.root
                .create(self.content_path())
                .map_err(|e| match e.kind() {
                    io::ErrorKind::AlreadyExists => CreateError::Occupied,
                    _ => CreateError::Internal(
                        anyhow::Error::new(e).context("failed to create content file"),
                    ),
                }),
        )?;
        let n = copy(meta.hash.verifier(rdr), &mut cont)
            .await
            .map_err(|e| match e.kind() {
                io::ErrorKind::InvalidData => CreateError::DigestMismatch,
                _ => {
                    CreateError::Internal(anyhow::Error::new(e).context("failed to write content"))
                }
            })?;
        if n != meta.size {
            return Err(CreateError::LengthMismatch {
                expected: meta.size,
                got: n,
            });
        }
        Ok(())
    }

    pub(super) async fn create_from_reader(
        &self,
        meta: Meta,
        rdr: impl Unpin + AsyncRead,
    ) -> Result<(), CreateError<anyhow::Error>> {
        self.create_from_reader_with(meta, rdr, dir_builder_defaults(&mut DirBuilder::new()))
            .await
    }

    pub(super) async fn create_json_with(
        &self,
        meta: Meta,
        val: &impl Serialize,
        dir_builder: &DirBuilder,
    ) -> Result<(), CreateError<anyhow::Error>> {
        let buf = serde_json::to_vec(val)
            .context("failed to encode value to JSON")
            .map_err(CreateError::Internal)?;
        self.create_from_reader_with(meta, buf.as_slice(), dir_builder)
            .await
    }

    pub(super) async fn create_json(
        &self,
        meta: Meta,
        val: &impl Serialize,
    ) -> Result<(), CreateError<anyhow::Error>> {
        self.create_json_with(meta, val, dir_builder_defaults(&mut DirBuilder::new()))
            .await
    }

    pub(super) async fn create_dir_with(
        &self,
        path: impl AsRef<Utf8Path>,
        dir_builder: &DirBuilder,
    ) -> Result<(), CreateError<anyhow::Error>> {
        let path = self.path(path);
        debug_assert_ne!(path, self.meta_path());
        debug_assert_ne!(path, self.content_path());

        self.root
            .create_dir_with(path, dir_builder)
            .map_err(|e| match e.kind() {
                io::ErrorKind::AlreadyExists => CreateError::Occupied,
                _ => CreateError::Internal(
                    anyhow::Error::new(e).context("failed to create directory"),
                ),
            })
    }

    pub(super) async fn create_dir(
        &self,
        path: impl AsRef<Utf8Path>,
    ) -> Result<(), CreateError<anyhow::Error>> {
        self.create_dir_with(path, dir_builder_defaults(&mut DirBuilder::new()))
            .await
    }

    pub(super) async fn read_dir(
        &self,
        path: impl AsRef<Utf8Path>,
    ) -> Result<ReadDir, GetError<anyhow::Error>> {
        self.root
            .read_dir(self.path(path))
            .await
            .map_err(|e| match e.kind() {
                io::ErrorKind::NotFound => GetError::NotFound,
                _ => GetError::Internal(anyhow::Error::new(e).context("failed to read directory")),
            })
    }

    /// Returns metadata of an entity
    pub async fn get_meta(&self) -> Result<Meta, GetError<anyhow::Error>> {
        let buf = self
            .root
            .read(self.meta_path())
            .await
            .map_err(|e| match e.kind() {
                io::ErrorKind::NotFound => GetError::NotFound,
                _ => GetError::Internal(anyhow::Error::new(e).context("failed to read metadata")),
            })?;
        serde_json::from_slice(&buf)
            .context("failed to decode metadata")
            .map_err(GetError::Internal)
    }

    /// Returns metadata of an entity and a reader of its contents.
    pub async fn get(&self) -> Result<(Meta, impl '_ + AsyncRead), GetError<anyhow::Error>> {
        try_join!(
            self.get_meta(),
            self.root
                .open(self.content_path())
                .map_err(|e| match e.kind() {
                    io::ErrorKind::NotFound => GetError::NotFound,
                    _ => GetError::Internal(
                        anyhow::Error::new(e).context("failed to open content file")
                    ),
                })
        )
    }

    pub async fn get_to_writer(
        &self,
        dst: &mut (impl Unpin + AsyncWrite),
    ) -> Result<Meta, GetToWriterError<anyhow::Error>> {
        let (meta, rdr) = self.get().await.map_err(GetToWriterError::Get)?;
        copy(rdr, dst).await.map_err(GetToWriterError::IO)?;
        Ok(meta)
    }
}

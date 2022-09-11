// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::io;
use std::os::unix::fs::DirBuilderExt;

use drawbridge_type::Meta;

use anyhow::{anyhow, Context};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use camino::{Utf8Path, Utf8PathBuf};
use cap_async_std::fs_utf8::{Dir, DirBuilder, ReadDir};
use drawbridge_type::digest::ContentDigest;
use futures::future::TryFutureExt;
use futures::io::copy;
use futures::try_join;
use futures::{AsyncRead, AsyncWrite};
use serde::{Deserialize, Serialize};
use tracing::{debug, trace};

const STORAGE_FAILURE_RESPONSE: (StatusCode, &str) =
    (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure");

#[derive(Debug)]
pub enum CreateError<E> {
    Occupied,
    LengthMismatch { expected: u64, got: u64 },
    DigestMismatch,
    Internal(E),
}

impl<E> IntoResponse for CreateError<E> {
    fn into_response(self) -> Response {
        match self {
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

#[derive(Clone, Debug, PartialEq, Eq)]
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

#[derive(Debug)]
pub(super) enum SymlinkError<E> {
    AlreadyExists,
    Internal(E),
}

#[derive(Copy, Clone, Debug)]
pub struct Entity<'a, P> {
    root: &'a Dir,
    prefix: P,
}

async fn create_verified(
    dir: &Dir,
    path: impl AsRef<Utf8Path>,
    hash: ContentDigest,
    size: u64,
    rdr: impl Unpin + AsyncRead,
) -> Result<(), CreateError<anyhow::Error>> {
    let mut file = dir.create(path).await.map_err(|e| match e.kind() {
        io::ErrorKind::AlreadyExists => CreateError::Occupied,
        _ => CreateError::Internal(anyhow::Error::new(e).context("failed to create file")),
    })?;
    match copy(hash.verifier(rdr), &mut file).await {
        Err(e) if e.kind() == io::ErrorKind::InvalidData => Err(CreateError::DigestMismatch),
        Err(e) => Err(CreateError::Internal(
            anyhow::Error::new(e).context("failed to write file"),
        )),
        Ok(n) if n != size => Err(CreateError::LengthMismatch {
            expected: size,
            got: n,
        }),
        Ok(_) => Ok(()),
    }
}

impl<'a> Entity<'a, &'static str> {
    pub fn new(root: &'a Dir) -> Self {
        Self { root, prefix: "" }
    }
}

impl<'a, P: AsRef<Utf8Path>> Entity<'a, P> {
    /// Returns a child [Entity] rooted at `path`.
    pub fn child(&self, path: impl AsRef<Utf8Path>) -> Entity<'a, Utf8PathBuf> {
        Entity {
            root: self.root,
            prefix: self.path(path),
        }
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

    pub(super) async fn create_from_reader(
        &self,
        meta: Meta,
        rdr: impl Unpin + AsyncRead,
    ) -> Result<(), CreateError<anyhow::Error>> {
        trace!(target: "app::store::Entity::create_from_reader", "create entity at `{}`", self.prefix.as_ref());
        let meta_json = serde_json::to_vec(&meta)
            .context("failed to encode metadata")
            .map_err(CreateError::Internal)?;
        try_join!(
            self.root
                .write(self.meta_path(), meta_json)
                .map_err(|e| match e.kind() {
                    io::ErrorKind::AlreadyExists => CreateError::Occupied,
                    _ => CreateError::Internal(
                        anyhow::Error::new(e).context("failed to write metadata"),
                    ),
                })
                .map_err(|e| {
                    debug!(target: "app::store::Entity::create_from_reader", "failed to create meta file `{:?}`", e);
                    e
                }),
            create_verified(self.root, self.content_path(), meta.hash, meta.size, rdr).map_err(|e| {
                debug!(target: "app::store::Entity::create_from_reader", "failed to create content file `{:?}`", e);
                e
            })
        )?;
        Ok(())
    }

    pub(super) async fn create_json(
        &self,
        meta: Meta,
        val: &impl Serialize,
    ) -> Result<(), CreateError<anyhow::Error>> {
        let buf = serde_json::to_vec(val)
            .context("failed to encode value to JSON")
            .map_err(CreateError::Internal)?;
        self.create_from_reader(meta, buf.as_slice()).await
    }

    pub(super) async fn create_dir(
        &self,
        path: impl AsRef<Utf8Path>,
    ) -> Result<(), CreateError<anyhow::Error>> {
        let path = self.path(path);
        debug_assert_ne!(path, self.meta_path());
        debug_assert_ne!(path, self.content_path());

        trace!(target: "app::store::Entity::create_dir", "create directory at `{path}`");
        self.root
            .create_dir_with(path, DirBuilder::new().mode(0o700))
            .map_err(|e| match e.kind() {
                io::ErrorKind::AlreadyExists => CreateError::Occupied,
                _ => CreateError::Internal(
                    anyhow::Error::new(e).context("failed to create directory"),
                ),
            })
            .map_err(|e| {
                debug!(target: "app::store::Entity::create_dir", "failed to create directory: `{:?}`", e);
                e
            })
    }

    pub(super) async fn symlink(
        &self,
        path: impl AsRef<Utf8Path>,
    ) -> Result<(), SymlinkError<anyhow::Error>> {
        let path = path.as_ref();
        let dest = {
            let mut dest = self.prefix.as_ref().components().peekable();
            let parents = path
                .components()
                .skip_while(|pc| match dest.peek() {
                    Some(dc) if pc == dc => {
                        // Components are equal, advance the `dest` iterator
                        _ = dest.next().unwrap();
                        true
                    }
                    _ => false,
                })
                .count();

            let dest = dest.collect::<Utf8PathBuf>();
            let mut buf = (0..parents - 1).fold(
                Utf8PathBuf::with_capacity(parents * "../".len() + dest.as_str().len()),
                |mut buf, _| {
                    buf.push("../");
                    buf
                },
            );
            buf.push(dest);
            buf
        };

        trace!(target: "app::store::entity", "create symlink to `{dest}` at `{path}`");
        self.root
            .symlink(dest, path)
            .await
            .map_err(|e| match e.kind() {
                io::ErrorKind::AlreadyExists => SymlinkError::AlreadyExists,
                _ => SymlinkError::Internal(anyhow::Error::new(e).context("failed to create symlink")),
            })
            .map_err(|e| {
                debug!(target: "app::store::Entity::symlink", "failed to create symlink: `{:?}`", e);
                e
            })
    }

    pub(super) async fn read_link(
        &self,
        path: impl AsRef<Utf8Path>,
    ) -> Result<(String, Entity<'a, Utf8PathBuf>), GetError<anyhow::Error>> {
        let path = self.path(path);
        let dest = self
            .root
            .read_link(&path)
            .await
            .map_err(|e| match e.kind() {
                io::ErrorKind::NotFound => GetError::NotFound,
                _ => GetError::Internal(anyhow::Error::new(e).context("failed to read link")),
            })?;
        let path = self
            .root
            .canonicalize(path.join(dest))
            .await
            .map_err(|e| match e.kind() {
                io::ErrorKind::NotFound => GetError::NotFound,
                _ => {
                    GetError::Internal(anyhow::Error::new(e).context("failed to canonicalize link"))
                }
            })?;
        let name = path.file_name().ok_or_else(|| {
            GetError::Internal(anyhow!("failed to read name of dereferenced file"))
        })?;
        Ok((name.into(), Entity::new(self.root).child(path)))
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

    /// Returns metadata of the entity.
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

    /// Returns contents of the entity as [AsyncRead].
    pub async fn get_content(&self) -> Result<impl '_ + AsyncRead, GetError<anyhow::Error>> {
        self.root
            .open(self.content_path())
            .map_err(|e| match e.kind() {
                io::ErrorKind::NotFound => GetError::NotFound,
                _ => {
                    GetError::Internal(anyhow::Error::new(e).context("failed to open content file"))
                }
            })
            .await
    }

    /// Reads contents of the entity.
    pub async fn read_content(&self) -> Result<Vec<u8>, GetError<anyhow::Error>> {
        self.root
            .read(self.content_path())
            .map_err(|e| match e.kind() {
                io::ErrorKind::NotFound => GetError::NotFound,
                _ => {
                    GetError::Internal(anyhow::Error::new(e).context("failed to read content file"))
                }
            })
            .await
    }

    /// Returns the contents of the entity as JSON.
    #[allow(single_use_lifetimes)]
    pub async fn get_content_json<T>(&self) -> Result<T, GetError<anyhow::Error>>
    where
        for<'de> T: Deserialize<'de>,
    {
        let buf = self.read_content().await?;
        serde_json::from_slice(&buf)
            .context("failed to decode content as JSON")
            .map_err(GetError::Internal)
    }

    /// Returns metadata of the entity and a reader of its contents.
    pub async fn get(&self) -> Result<(Meta, impl '_ + AsyncRead), GetError<anyhow::Error>> {
        try_join!(self.get_meta(), self.get_content())
    }

    /// Returns metadata of the entity and writes its contents into `dst`.
    pub async fn get_to_writer(
        &self,
        dst: &mut (impl Unpin + AsyncWrite),
    ) -> Result<Meta, GetToWriterError<anyhow::Error>> {
        let (meta, rdr) = self.get().await.map_err(GetToWriterError::Get)?;
        _ = copy(rdr, dst).await.map_err(GetToWriterError::IO)?;
        // TODO: Validate size
        Ok(meta)
    }
}

// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]
#![feature(str_split_as_str)]

mod builder;
mod handle;
mod repos;
mod tags;
mod trees;

pub use builder::*;
pub(crate) use handle::*;
pub use repos::*;
pub use tags::*;
pub use trees::*;

use std::fmt::Display;
use std::sync::Arc;

use drawbridge_store::{Filesystem, Get};
use drawbridge_type::{RepositoryName, TagName, TreePath};

use axum::http::StatusCode;
use tokio::sync::RwLock;

pub type RepoStore = RwLock<Filesystem<RepositoryName>>;
pub type TagStore = RwLock<Filesystem<(RepositoryName, TagName)>>;
pub type TreeStore = RwLock<Filesystem<(RepositoryName, TagName, TreePath)>>;

pub async fn assert_repo(
    repos: Arc<RepoStore>,
    repo: RepositoryName,
) -> Result<(), (StatusCode, &'static str)> {
    #[inline]
    fn err_map(e: impl Display) -> (StatusCode, &'static str) {
        eprintln!("failed to check repository existence: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to check repository existence",
        )
    }
    if !repos
        .read()
        .await
        .contains(repo.clone())
        .await
        .map_err(err_map)?
    {
        Err((StatusCode::NOT_FOUND, "Repository does not exist"))
    } else {
        Ok(())
    }
}

pub async fn assert_tag(
    tags: Arc<TagStore>,
    repo: RepositoryName,
    tag: TagName,
) -> Result<(), (StatusCode, &'static str)> {
    #[inline]
    fn err_map(e: impl Display) -> (StatusCode, &'static str) {
        eprintln!("failed to check tag existence: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to check tag existence",
        )
    }
    if !tags
        .read()
        .await
        .contains((repo.clone(), tag.clone()))
        .await
        .map_err(err_map)?
    {
        Err((StatusCode::NOT_FOUND, "Tag does not exist"))
    } else {
        Ok(())
    }
}

// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_repo, assert_tag, RepoStore, TagStore, TreeStore};

use std::sync::Arc;

use drawbridge_store::{Get, GetError, GetToWriterError};
use drawbridge_type::{RepositoryName, TagName, TreePath};

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Extension;

pub async fn get(
    Extension(repos): Extension<Arc<RepoStore>>,
    Extension(tags): Extension<Arc<TagStore>>,
    Extension(trees): Extension<Arc<TreeStore>>,
    Extension(repo): Extension<RepositoryName>,
    Extension(tag): Extension<TagName>,
    Extension(path): Extension<TreePath>,
) -> impl IntoResponse {
    assert_repo(repos, repo.clone()).await?;
    assert_tag(tags, repo.clone(), tag.clone()).await?;

    // TODO: Stream body
    // https://github.com/profianinc/drawbridge/issues/56
    let mut body = vec![];
    let meta = trees
        .read()
        .await
        .get_to_writer((repo, tag, path), &mut body)
        .await
        .map_err(|e| match e {
            GetToWriterError::Get(GetError::NotFound) => {
                (StatusCode::NOT_FOUND, "Tag does not exist")
            }
            GetToWriterError::Get(GetError::Internal(e)) => {
                eprintln!("Failed to get repository: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
            }
            GetToWriterError::IO(e) => {
                eprintln!("Failed to read repository contents: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
            }
        })?;
    Ok::<_, (_, _)>((meta, body))
}

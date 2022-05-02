// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_repo, assert_tag, RepoStore, TagStore, TreeStore};

use std::sync::Arc;

use drawbridge_store::{Get, GetError};
use drawbridge_type::{RepositoryName, TagName, TreePath};

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Extension;

pub async fn head(
    Extension(repos): Extension<Arc<RepoStore>>,
    Extension(tags): Extension<Arc<TagStore>>,
    Extension(trees): Extension<Arc<TreeStore>>,
    Extension(repo): Extension<RepositoryName>,
    Extension(tag): Extension<TagName>,
    Extension(path): Extension<TreePath>,
) -> impl IntoResponse {
    assert_repo(repos, repo.clone()).await?;
    assert_tag(tags, repo.clone(), tag.clone()).await?;

    trees
        .read()
        .await
        .get_meta((repo, tag, path))
        .await
        .map_err(|e| match e {
            GetError::NotFound => (StatusCode::NOT_FOUND, "Path does not exist"),
            GetError::Internal(e) => {
                eprintln!("Failed to get path metadata: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
            }
        })
        .map(|meta| (meta, ()))
}

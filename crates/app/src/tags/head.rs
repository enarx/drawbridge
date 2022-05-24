// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{assert_repo, RepoStore, TagStore};

use std::sync::Arc;

use drawbridge_store::{Get, GetError};
use drawbridge_type::{RepositoryName, TagName};

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Extension;

pub async fn head(
    Extension(repos): Extension<Arc<RepoStore>>,
    Extension(tags): Extension<Arc<TagStore>>,
    Extension(repo): Extension<RepositoryName>,
    Extension(name): Extension<TagName>,
) -> impl IntoResponse {
    assert_repo(repos, repo.clone()).await?;

    tags.read()
        .await
        .get_meta((repo, name))
        .await
        .map_err(|e| match e {
            GetError::NotFound => (StatusCode::NOT_FOUND, "Tag does not exist"),
            GetError::Internal(e) => {
                eprintln!("Failed to get tag metadata: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
            }
        })
        .map(|meta| (meta, ()))
}

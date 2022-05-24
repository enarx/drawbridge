// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{assert_repo, RepoStore, TagStore};

use std::sync::Arc;

use drawbridge_store::Keys;
use drawbridge_type::RepositoryName;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use futures::TryStreamExt;

pub async fn query(
    Extension(repos): Extension<Arc<RepoStore>>,
    Extension(tags): Extension<Arc<TagStore>>,
    Extension(repo): Extension<RepositoryName>,
) -> impl IntoResponse {
    assert_repo(repos, repo.clone()).await?;

    tags.read()
        .await
        .keys()
        .await
        .try_filter_map(move |(r, n)| {
            let repo = repo.clone();
            async move { Ok(if r == repo { Some(n.to_string()) } else { None }) }
        })
        .try_collect::<Vec<String>>()
        .await
        .map_err(|e| {
            eprintln!("Failed to get tag name: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "")
        })
        .map(Json)
}

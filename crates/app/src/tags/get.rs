// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{assert_repo, RepoStore, TagStore};

use std::sync::Arc;

use drawbridge_store::{Get, GetError, GetToWriterError};
use drawbridge_type::{RepositoryName, TagName};

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Extension;

pub async fn get(
    Extension(repos): Extension<Arc<RepoStore>>,
    Extension(tags): Extension<Arc<TagStore>>,
    Extension(repo): Extension<RepositoryName>,
    Extension(name): Extension<TagName>,
) -> impl IntoResponse {
    assert_repo(repos, repo.clone()).await?;

    // TODO: Stream body
    // https://github.com/profianinc/drawbridge/issues/56
    let mut body = vec![];
    let meta = tags
        .read()
        .await
        .get_to_writer((repo, name), &mut body)
        .await
        .map_err(|e| match e {
            GetToWriterError::Get(GetError::NotFound) => {
                (StatusCode::NOT_FOUND, "Tag does not exist")
            }
            GetToWriterError::Get(GetError::Internal(e)) => {
                eprintln!("Failed to get tag: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
            }
            GetToWriterError::IO(e) => {
                eprintln!("Failed to read tag contents: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
            }
        })?;
    Ok::<_, (_, _)>((meta, body))
}

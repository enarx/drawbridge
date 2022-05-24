// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::RepoStore;

use std::sync::Arc;

use drawbridge_store::{Get, GetError, GetToWriterError};
use drawbridge_type::RepositoryName;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Extension;

pub async fn get(
    Extension(repos): Extension<Arc<RepoStore>>,
    Extension(name): Extension<RepositoryName>,
) -> impl IntoResponse {
    // TODO: Stream body
    // https://github.com/profianinc/drawbridge/issues/56
    let mut body = vec![];
    let meta = repos
        .read()
        .await
        .get_to_writer(name, &mut body)
        .await
        .map_err(|e| match e {
            GetToWriterError::Get(GetError::NotFound) => {
                (StatusCode::NOT_FOUND, "Repository does not exist")
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

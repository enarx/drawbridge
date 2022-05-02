// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use crate::RepoStore;

use std::sync::Arc;

use drawbridge_store::{Get, GetError};
use drawbridge_type::RepositoryName;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Extension;

pub async fn head(
    Extension(repos): Extension<Arc<RepoStore>>,
    Extension(name): Extension<RepositoryName>,
) -> impl IntoResponse {
    repos
        .read()
        .await
        .get_meta(name)
        .await
        .map_err(|e| match e {
            GetError::NotFound => (StatusCode::NOT_FOUND, "Repository does not exist"),
            GetError::Internal(e) => {
                eprintln!("Failed to get repository metadata: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
            }
        })
        .map(|meta| (meta, ()))
}

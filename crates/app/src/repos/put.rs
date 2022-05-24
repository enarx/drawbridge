// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::RepoStore;

use std::sync::Arc;

use drawbridge_store::{Create, CreateError, CreateFromReaderError};
use drawbridge_type::{Meta, RepositoryConfig, RepositoryName};

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};

pub async fn put(
    Extension(repos): Extension<Arc<RepoStore>>,
    Extension(name): Extension<RepositoryName>,
    Meta { hash, size, mime }: Meta,
    Json(config): Json<RepositoryConfig>,
) -> impl IntoResponse {
    let buf = serde_json::to_vec(&config).unwrap();
    if buf.len() as u64 != size {
        // TODO: Report error location
        // https://github.com/profianinc/drawbridge/issues/97
        return Err((StatusCode::BAD_REQUEST, "Invalid repository encoding, make sure the object is minified and keys are sorted lexicographically".into_response()));
    }
    repos
        .write()
        .await
        .create_from_reader(name.clone(), mime.clone(), hash.verifier(buf.as_slice()))
        .await
        .map_err(|e| match e {
            CreateFromReaderError::IO(e) => {
                eprintln!("Failed to stream repository contents: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Storage backend failure".into_response(),
                )
            }
            CreateFromReaderError::Create(CreateError::Occupied) => (
                StatusCode::CONFLICT,
                "Repository already exists".into_response(),
            ),
            CreateFromReaderError::Create(CreateError::Internal(e)) => {
                eprintln!("Failed to create repository: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Storage backend failure".into_response(),
                )
            }
        })
        .map(|_| StatusCode::CREATED)
}

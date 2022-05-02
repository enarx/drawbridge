// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use crate::RepoStore;

use std::sync::Arc;

use drawbridge_store::{
    Create, CreateError, CreateFromReaderError, Get, GetError, GetToWriterError,
};
use drawbridge_type::repository::{Config, Name};
use drawbridge_type::Meta;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};

pub async fn head(
    Extension(repos): Extension<Arc<RepoStore>>,
    Extension(name): Extension<Name>,
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

pub async fn get(
    Extension(repos): Extension<Arc<RepoStore>>,
    Extension(name): Extension<Name>,
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

pub async fn put(
    Extension(repos): Extension<Arc<RepoStore>>,
    Extension(name): Extension<Name>,
    Meta { hash, size, mime }: Meta,
    Json(config): Json<Config>,
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

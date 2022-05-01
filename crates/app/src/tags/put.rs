// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_repo, RepoStore, TagStore};

use std::sync::Arc;

use drawbridge_jose::jws::Jws;
use drawbridge_jose::MediaTyped;
use drawbridge_store::{Create, CreateError, CreateFromReaderError};
use drawbridge_type::{Meta, RepositoryName, TagEntry, TagName, TreeEntry};

use axum::body::Body;
use axum::extract::RequestParts;
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use axum::{Extension, Json};

pub async fn put(
    Extension(repos): Extension<Arc<RepoStore>>,
    Extension(tags): Extension<Arc<TagStore>>,
    Extension(repo): Extension<RepositoryName>,
    Extension(name): Extension<TagName>,
    Meta { hash, size, mime }: Meta,
    req: Request<Body>,
) -> impl IntoResponse {
    assert_repo(repos, repo.clone())
        .await
        .map_err(|(code, e)| (code, e.into_response()))?;

    let mut req = RequestParts::new(req);
    let tag = match mime.to_string().as_str() {
        TreeEntry::TYPE => req.extract().await.map(|Json(v)| TagEntry::Unsigned(v)),
        Jws::TYPE => req.extract().await.map(|Json(v)| TagEntry::Signed(v)),
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Invalid content type".into_response(),
            ))
        }
    }
    .map_err(|e| (StatusCode::BAD_REQUEST, e.into_response()))?;

    let buf = serde_json::to_vec(&tag).unwrap();
    if buf.len() as u64 != size {
        // TODO: Report error location
        // https://github.com/profianinc/drawbridge/issues/97
        return Err((StatusCode::BAD_REQUEST, "Invalid tag encoding, make sure the object is minified and keys are sorted lexicographically".into_response()));
    }
    tags.write()
        .await
        .create_from_reader((repo, name), mime.clone(), hash.verifier(buf.as_slice()))
        .await
        .map_err(|e| match e {
            CreateFromReaderError::IO(e) => {
                eprintln!("Failed to stream tag contents: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Storage backend failure".into_response(),
                )
            }
            CreateFromReaderError::Create(CreateError::Occupied) => {
                (StatusCode::CONFLICT, "Tag already exists".into_response())
            }
            CreateFromReaderError::Create(CreateError::Internal(e)) => {
                eprintln!("Failed to create tag: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Storage backend failure".into_response(),
                )
            }
        })
        .map(|_| StatusCode::CREATED)
}

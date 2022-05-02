// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_repo, assert_tag, RepoStore, TagStore, TreeStore};

use std::sync::Arc;

use drawbridge_store::{Create, CreateError, CreateFromReaderError};
use drawbridge_type::{Meta, RepositoryName, TagName, TreeDirectory, TreePath};

use axum::body::Body;
use axum::extract::{BodyStream, RequestParts};
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use futures::{io, TryStreamExt};

#[allow(clippy::too_many_arguments)]
pub async fn put(
    Extension(repos): Extension<Arc<RepoStore>>,
    Extension(tags): Extension<Arc<TagStore>>,
    Extension(trees): Extension<Arc<TreeStore>>,
    Extension(repo): Extension<RepositoryName>,
    Extension(tag): Extension<TagName>,
    Extension(path): Extension<TreePath>,
    Meta { hash, size, mime }: Meta,
    req: Request<Body>,
) -> impl IntoResponse {
    assert_repo(repos, repo.clone())
        .await
        .map_err(|(code, e)| (code, e.into_response()))?;
    assert_tag(tags, repo.clone(), tag.clone())
        .await
        .map_err(|(code, e)| (code, e.into_response()))?;

    let mut req = RequestParts::new(req);
    match mime.to_string().as_str() {
        TreeDirectory::TYPE => {
            let dir = req
                .extract()
                .await
                .map(|Json(v): Json<TreeDirectory>| v)
                .map_err(|e| (StatusCode::BAD_REQUEST, e.into_response()))?;
            let buf = serde_json::to_vec(&dir).unwrap();
            if buf.len() as u64 != size {
                // TODO: Report error location
                // https://github.com/profianinc/drawbridge/issues/97
                return Err((StatusCode::BAD_REQUEST, "Invalid directory encoding, make sure the object is minified and keys are sorted lexicographically".into_response()));
            }
            trees.write()
                .await
                .create_from_reader((repo, tag, path), mime.clone(), hash.verifier(buf.as_slice()))
                .await
        }
        _ => {
            // TODO: Validate node hash against parents' expected values
            // https://github.com/profianinc/drawbridge/issues/77
            let body = req
                .extract::<BodyStream>()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, e.into_response()))?
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e));
            trees.write()
                .await
                .create_from_reader((repo, tag, path), mime.clone(), hash.verifier(body.into_async_read()))
                .await
        }
    }
    .map_err(|e| match e {
        CreateFromReaderError::IO(e) => {
            eprintln!("Failed to stream path contents: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Storage backend failure".into_response(),
            )
        }
        CreateFromReaderError::Create(CreateError::Occupied) => {
            (StatusCode::CONFLICT, "Path already exists".into_response())
        }
        CreateFromReaderError::Create(CreateError::Internal(e)) => {
            eprintln!("Failed to create path: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Storage backend failure".into_response(),
            )
        }
    })
    .map(|_| StatusCode::CREATED)
}

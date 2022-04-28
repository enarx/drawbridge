// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_repo, assert_tag, RepoStore, TagStore, TreeStore};

use std::sync::Arc;

use drawbridge_store::{
    Create, CreateError, CreateFromReaderError, Get, GetError, GetToWriterError,
};
use drawbridge_type::tree::{Directory, Path};
use drawbridge_type::{repository, tag, RequestMeta};

use axum::body::Body;
use axum::extract::{BodyStream, RequestParts};
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use futures::{io, TryStreamExt};

pub async fn head(
    Extension(repos): Extension<Arc<RepoStore>>,
    Extension(tags): Extension<Arc<TagStore>>,
    Extension(trees): Extension<Arc<TreeStore>>,
    Extension(repo): Extension<repository::Name>,
    Extension(tag): Extension<tag::Name>,
    Extension(path): Extension<Path>,
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

pub async fn get(
    Extension(repos): Extension<Arc<RepoStore>>,
    Extension(tags): Extension<Arc<TagStore>>,
    Extension(trees): Extension<Arc<TreeStore>>,
    Extension(repo): Extension<repository::Name>,
    Extension(tag): Extension<tag::Name>,
    Extension(path): Extension<Path>,
) -> impl IntoResponse {
    assert_repo(repos, repo.clone()).await?;
    assert_tag(tags, repo.clone(), tag.clone()).await?;

    // TODO: Stream body
    // https://github.com/profianinc/drawbridge/issues/56
    let mut body = vec![];
    let meta = trees
        .read()
        .await
        .get_to_writer((repo, tag, path), &mut body)
        .await
        .map_err(|e| match e {
            GetToWriterError::Get(GetError::NotFound) => {
                (StatusCode::NOT_FOUND, "Tag does not exist")
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

#[allow(clippy::too_many_arguments)]
pub async fn put(
    Extension(repos): Extension<Arc<RepoStore>>,
    Extension(tags): Extension<Arc<TagStore>>,
    Extension(trees): Extension<Arc<TreeStore>>,
    Extension(repo): Extension<repository::Name>,
    Extension(tag): Extension<tag::Name>,
    Extension(path): Extension<Path>,
    RequestMeta { hash, size, mime }: RequestMeta,
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
        Directory::TYPE => {
            let dir = req
                .extract()
                .await
                .map(|Json(v): Json<Directory>| v)
                .map_err(|e| (StatusCode::BAD_REQUEST, e.into_response()))?;
            let buf = serde_json::to_vec(&dir).unwrap();
            if let Some(size) = size {
                if buf.len() as u64 != size {
                    // TODO: Report error location
                    // https://github.com/profianinc/drawbridge/issues/97
                    return Err((StatusCode::BAD_REQUEST, "Invalid directory encoding, make sure the object is minified and keys are sorted lexicographically".into_response()));
                }
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
            // TODO: Validate body size
            // https://github.com/profianinc/drawbridge/issues/96
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

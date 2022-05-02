// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_repo, RepoStore, TagStore};

use std::sync::Arc;

use drawbridge_jose::jws::Jws;
use drawbridge_store::{
    Create, CreateError, CreateFromReaderError, Get, GetError, GetToWriterError, Keys,
};
use drawbridge_type::tag::{Entry, Name};
use drawbridge_type::{repository, tree, Meta};

use axum::body::Body;
use axum::extract::RequestParts;
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use futures::TryStreamExt;

pub async fn query(
    Extension(repos): Extension<Arc<RepoStore>>,
    Extension(tags): Extension<Arc<TagStore>>,
    Extension(repo): Extension<repository::Name>,
) -> impl IntoResponse {
    assert_repo(repos, repo.clone()).await?;

    tags.read()
        .await
        .keys()
        .await
        .try_filter_map(move |(r, n)| {
            let repo = repo.clone();
            async move { Ok(if r == repo { Some(n.into()) } else { None }) }
        })
        .try_collect::<Vec<String>>()
        .await
        .map_err(|e| {
            eprintln!("Failed to get tag name: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "")
        })
        .map(Json)
}

pub async fn head(
    Extension(repos): Extension<Arc<RepoStore>>,
    Extension(tags): Extension<Arc<TagStore>>,
    Extension(repo): Extension<repository::Name>,
    Extension(name): Extension<Name>,
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

pub async fn get(
    Extension(repos): Extension<Arc<RepoStore>>,
    Extension(tags): Extension<Arc<TagStore>>,
    Extension(repo): Extension<repository::Name>,
    Extension(name): Extension<Name>,
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

pub async fn put(
    Extension(repos): Extension<Arc<RepoStore>>,
    Extension(tags): Extension<Arc<TagStore>>,
    Extension(repo): Extension<repository::Name>,
    Extension(name): Extension<Name>,
    Meta { hash, size, mime }: Meta,
    req: Request<Body>,
) -> impl IntoResponse {
    assert_repo(repos, repo.clone())
        .await
        .map_err(|(code, e)| (code, e.into_response()))?;

    let mut req = RequestParts::new(req);
    let tag = match mime.to_string().as_str() {
        tree::Entry::TYPE => req.extract().await.map(|Json(v)| Entry::Unsigned(v)),
        Jws::TYPE => req.extract().await.map(|Json(v)| Entry::Signed(v)),
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

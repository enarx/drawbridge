// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::Store;

use std::sync::Arc;

use drawbridge_type::{Meta, RepositoryName, TagName, TreeDirectory, TreePath};

use axum::body::Body;
use axum::extract::{BodyStream, RequestParts};
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use futures::{io, TryStreamExt};

pub async fn put(
    Extension(store): Extension<Arc<Store>>,
    Extension(repo): Extension<RepositoryName>,
    Extension(tag): Extension<TagName>,
    Extension(path): Extension<TreePath>,
    meta: Meta,
    req: Request<Body>,
) -> impl IntoResponse {
    let mut req = RequestParts::new(req);
    match meta.mime.to_string().as_str() {
        TreeDirectory::TYPE => {
            let dir = req
                .extract()
                .await
                .map(|Json(v)| v)
                .map_err(|e| (StatusCode::BAD_REQUEST, e).into_response())?;
            store
                .repository(&repo)
                .tag(&tag)
                .tree_node(&path)
                .create_directory(meta, &dir)
                .await
        }
        _ => {
            let body = req
                .extract::<BodyStream>()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, e).into_response())?
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e));
            store
                .repository(&repo)
                .tag(&tag)
                .tree_node(&path)
                .create_file(meta, body.into_async_read())
                .await
        }
    }
    .map_err(|e| {
        eprintln!(
            "Failed to PUT path `{}` on tag `{}` of repository `{}`: {:?}",
            path, tag, repo, e
        );
        e
    })
    .map_err(IntoResponse::into_response)
    .map(|_| StatusCode::CREATED)
}

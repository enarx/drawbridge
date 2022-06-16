// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::Store;

use drawbridge_type::{Meta, TreeContext, TreeDirectory};

use async_std::sync::Arc;
use axum::body::Body;
use axum::extract::{BodyStream, RequestParts};
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use futures::{io, TryStreamExt};
use log::warn;

pub async fn put(
    Extension(store): Extension<Arc<Store>>,
    tree: TreeContext,
    meta: Meta,
    req: Request<Body>,
) -> impl IntoResponse {
    if meta.hash.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "At least one content digest value must be specified",
        )
            .into_response());
    }

    let mut req = RequestParts::new(req);
    match meta.mime.to_string().as_str() {
        TreeDirectory::<()>::TYPE => {
            let dir = req
                .extract()
                .await
                .map(|Json(v)| v)
                .map_err(|e| (StatusCode::BAD_REQUEST, e).into_response())?;
            store.tree(&tree).create_directory(meta, &dir).await
        }
        _ => {
            let body = req
                .extract::<BodyStream>()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, e).into_response())?
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e));
            store
                .tree(&tree)
                .create_file(meta, body.into_async_read())
                .await
        }
    }
    .map_err(|e| {
        warn!(target: "app::trees::put", "failed for `{tree}`: {:?}", e);
        e
    })
    .map_err(IntoResponse::into_response)
    .map(|_| StatusCode::CREATED)
}

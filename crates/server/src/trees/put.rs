// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::{OidcClaims, ScopeContext, ScopeLevel, Store};

use drawbridge_type::{Meta, TreeContext, TreeDirectory};

use async_std::sync::Arc;
use axum::body::Body;
use axum::extract::{BodyStream, RequestParts};
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use futures::{io, TryStreamExt};
use tracing::{debug, trace};

pub async fn put(
    Extension(ref store): Extension<Arc<Store>>,
    claims: OidcClaims,
    cx: TreeContext,
    meta: Meta,
    req: Request<Body>,
) -> impl IntoResponse {
    trace!(target: "app::trees::put", "called for `{cx}`");

    if meta.hash.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "At least one content digest value must be specified",
        )
            .into_response());
    }

    let user = claims
        .assert_user(
            store,
            &cx.tag.repository.owner,
            ScopeContext::Tag,
            ScopeLevel::Write,
        )
        .await
        .map_err(IntoResponse::into_response)?;

    let mut req = RequestParts::new(req);
    let tag = user.repository(&cx.tag.repository.name).tag(&cx.tag.name);
    match meta.mime.to_string().as_str() {
        TreeDirectory::<()>::TYPE => {
            let dir = req
                .extract()
                .await
                .map(|Json(v)| v)
                .map_err(|e| (StatusCode::BAD_REQUEST, e).into_response())?;
            tag.create_directory_node(&cx.path, meta, &dir).await
        }
        _ => {
            let body = req
                .extract::<BodyStream>()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, e).into_response())?
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e));
            tag.create_file_node(&cx.path, meta, body.into_async_read())
                .await
        }
    }
    .map_err(|e| {
        debug!(target: "app::trees::put", "failed for `{cx}`: {:?}", e);
        e.into_response()
    })
    .map(|_| StatusCode::CREATED)
}

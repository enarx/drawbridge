// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::{OidcClaims, Store};

use drawbridge_type::{Meta, TreeContext, TreeDirectory};

use async_std::sync::Arc;
use axum::body::Body;
use axum::extract::{BodyStream, RequestParts};
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use futures::{io, TryStreamExt};
use log::debug;

pub async fn put(
    Extension(store): Extension<Arc<Store>>,
    claims: OidcClaims,
    cx: TreeContext,
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

    let (oidc_cx, user) = claims
        .get_user(&store)
        .await
        .map_err(IntoResponse::into_response)?;
    if oidc_cx != cx.tag.repository.owner {
        return Err((
            StatusCode::UNAUTHORIZED,
            format!(
                "You are logged in as `{oidc_cx}`, please relogin as `{}` to access `{cx}`",
                cx.tag.repository.owner
            ),
        )
            .into_response());
    }

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

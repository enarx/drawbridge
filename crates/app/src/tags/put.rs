// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::{OidcClaims, Store};

use drawbridge_jose::jws::Jws;
use drawbridge_jose::MediaTyped;
use drawbridge_type::{Meta, TagContext, TagEntry, TreeEntry};

use async_std::sync::Arc;
use axum::body::Body;
use axum::extract::RequestParts;
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use log::debug;

pub async fn put(
    Extension(store): Extension<Arc<Store>>,
    claims: OidcClaims,
    cx: TagContext,
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
    if oidc_cx != cx.repository.owner {
        return Err((
            StatusCode::UNAUTHORIZED,
            format!(
                "You are logged in as `{oidc_cx}`, please relogin as `{}` to access `{cx}`",
                cx.repository.owner
            ),
        )
            .into_response());
    }

    let mut req = RequestParts::new(req);
    let entry = match meta.mime.to_string().as_str() {
        TreeEntry::<()>::TYPE => req.extract().await.map(|Json(v)| TagEntry::Unsigned(v)),
        Jws::TYPE => req.extract().await.map(|Json(v)| TagEntry::Signed(v)),
        _ => return Err((StatusCode::BAD_REQUEST, "Invalid content type").into_response()),
    }
    .map_err(|e| (StatusCode::BAD_REQUEST, e).into_response())?;
    user.repository(&cx.repository.name)
        .create_tag(&cx.name, meta, &entry)
        .await
        .map_err(|e| {
            debug!(target: "app::tags::put", "failed for `{cx}`: {:?}", e);
            e.into_response()
        })
        .map(|_| StatusCode::CREATED)
}

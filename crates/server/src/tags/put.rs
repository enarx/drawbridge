// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::{OidcClaims, ScopeContext, ScopeLevel, Store};

use drawbridge_jose::jws::Jws;
use drawbridge_jose::MediaTyped;
use drawbridge_type::{Meta, TagContext, TagEntry, TreeEntry};

use async_std::sync::Arc;
use axum::body::Body;
use axum::extract::RequestParts;
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use tracing::{debug, trace};

pub async fn put(
    Extension(store): Extension<Arc<Store>>,
    claims: OidcClaims,
    cx: TagContext,
    meta: Meta,
    req: Request<Body>,
) -> impl IntoResponse {
    trace!(target: "app::tags::put", "called for `{cx}`");

    if meta.hash.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "At least one content digest value must be specified",
        )
            .into_response());
    }

    let user = claims
        .assert_user(
            &store,
            &cx.repository.owner,
            ScopeContext::Tag,
            ScopeLevel::Write,
        )
        .await
        .map_err(IntoResponse::into_response)?;

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
